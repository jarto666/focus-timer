import { JournalHealth, MAX_RECORDS_PER_PAGE } from '@focus-timer/device-protocol';

import { DeviceClient } from './device-client';
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
  utcNowMs: number,
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
  await client.setClockAnchor(utcNowMs, operation);
  const status = await client.getStatus(operation);

  if (status.journal.health === JournalHealth.Unavailable) {
    return {
      deviceId,
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
