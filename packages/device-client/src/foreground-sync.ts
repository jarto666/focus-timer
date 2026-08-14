import {
  JournalHealth,
  MAX_RECORDS_PER_PAGE,
  ProtocolErrorCode,
} from '@focus-timer/device-protocol';

import { DeviceClient, DeviceClientError } from './device-client';
import type {
  DeviceCandidate,
  DeviceTransport,
  DeviceTransportOperation,
} from './device-transport';
import {
  bytesToHex,
  hexToBytes,
  type HistoryCompleteness,
  type JournalCursor,
  type SessionRepository,
  type KnownDeviceRecord,
} from './session-repository';

const MAX_PAGES_PER_SYNC = 64;

export class ForegroundSyncError extends Error {
  override readonly name = 'ForegroundSyncError';
}

export type ForegroundSyncResult = Readonly<{
  client: DeviceClient;
  deviceId: string;
  device: KnownDeviceRecord;
  status: Awaited<ReturnType<DeviceClient['getStatus']>>;
  pagesCommitted: number;
  recordsReceived: number;
  cursor: JournalCursor | null;
  journalAvailable: boolean;
}>;

/**
 * Foreground correctness path: connect, handshake, set volatile wall time,
 * read status, then durably catch up each stateless journal page.
 */
export async function synchronizeForeground(
  transport: DeviceTransport,
  candidate: DeviceCandidate,
  repository: SessionRepository,
  utcNowMs: () => number,
  operation: DeviceTransportOperation,
): Promise<ForegroundSyncResult> {
  const client = new DeviceClient(transport);
  await transport.connect(candidate, operation);
  const hello = await client.handshake(operation);
  const deviceId = bytesToHex(hello.deviceId);
  const device: KnownDeviceRecord = {
    deviceId,
    transportId: candidate.transportId,
    productName: hello.productName,
    firmwareVersion: hello.firmwareVersion,
    protocolVersion: hello.supportedVersion,
  };
  await repository.rememberDevice(device);
  // Read wall time only after the link and version handshake are ready. A
  // timestamp captured before BLE connection setup can already be older than
  // the device's current anchor by the time this request reaches firmware,
  // causing a healthy foreground reconnect to be rejected as backwards time.
  try {
    await client.setClockAnchor(utcNowMs(), operation);
  } catch (error) {
    // A volatile wall-clock refresh is optional metadata, not a prerequisite
    // for read-only status/history sync. Crystal drift or a legitimate phone
    // clock correction can put a later anchor fractionally behind the device's
    // extrapolated UTC. Firmware correctly rejects that backwards update and
    // preserves its previous anchor; the companion must still catch up the
    // durable journal instead of turning the whole reconnect into a failure.
    const rejectedClockUpdate =
      error instanceof DeviceClientError &&
      error.code === 'remote-error' &&
      error.details.protocolErrorCode === ProtocolErrorCode.InvalidField;
    if (!rejectedClockUpdate) throw error;
  }
  const status = await client.getStatus(operation);

  if (status.journal.health === JournalHealth.Unavailable) {
    return {
      deviceId,
      client,
      device,
      status,
      pagesCommitted: 0,
      recordsReceived: 0,
      cursor: await repository.loadActiveCursor(deviceId),
      journalAvailable: false,
    };
  }

  let cursor = await repository.loadActiveCursor(deviceId);
  const currentEpoch = bytesToHex(status.journal.epoch);
  if (
    cursor?.journalEpoch === currentEpoch &&
    cursor.latestSequence === (status.journal.latestSequence ?? null) &&
    cursor.afterSequence === (status.journal.latestSequence ?? 0)
  ) {
    return {
      deviceId,
      client,
      device,
      status,
      pagesCommitted: 0,
      recordsReceived: 0,
      cursor,
      journalAvailable: true,
    };
  }

  let requestedEpoch = cursor?.journalEpoch;
  let requestedAfterSequence = cursor?.afterSequence ?? 0;
  let pagesCommitted = 0;
  let recordsReceived = 0;

  for (let pageNumber = 0; pageNumber < MAX_PAGES_PER_SYNC; pageNumber += 1) {
    const page = await client.getSessionPage(
      {
        ...(requestedEpoch === undefined ? {} : { journalEpoch: hexToBytes(requestedEpoch) }),
        afterSequence: requestedAfterSequence,
        limit: MAX_RECORDS_PER_PAGE,
      },
      operation,
    );
    const completeness: HistoryCompleteness = page.epochChanged
      ? 'changed-epoch'
      : page.gap
        ? 'retention-gap'
        : (cursor?.completeness ?? 'complete');
    cursor = await repository.commitPage({
      deviceId,
      requestedAfterSequence,
      page,
      completeness,
    });
    pagesCommitted += 1;
    recordsReceived += page.records.length;

    if (page.continuationAfterSequence === undefined) {
      return {
        deviceId,
        client,
        device,
        status,
        pagesCommitted,
        recordsReceived,
        cursor,
        journalAvailable: true,
      };
    }
    if (
      page.continuationAfterSequence <= requestedAfterSequence &&
      !page.epochChanged &&
      !page.gap
    ) {
      throw new ForegroundSyncError('Timer returned a non-advancing continuation cursor');
    }
    requestedEpoch = bytesToHex(page.journalEpoch);
    requestedAfterSequence = page.continuationAfterSequence;
  }

  throw new ForegroundSyncError(
    `Timer exceeded ${MAX_PAGES_PER_SYNC} pages in one foreground sync`,
  );
}
