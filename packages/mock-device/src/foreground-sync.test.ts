import { describe, expect, it } from 'vitest';

import {
  DeviceClientError,
  DeviceTransportError,
  bytesToHex,
  nextCursorAfterSequence,
  synchronizeForeground,
  type JournalCursor,
  type KnownDeviceRecord,
  type PageCommit,
  type SessionRepository,
  type StoredSessionRecord,
} from '@focus-timer/device-client';
import { decodeRequest } from '@focus-timer/device-protocol';

import { DeterministicMockDevice } from './mock-device';
import { createMockTransport } from './mock-transport';
import { createProtocolMockTransport, createProtocolResponder } from './protocol-responder';
import { getMockScenario } from './scenarios';

const operation = { timeoutMs: 2_000 } as const;
const nowUtcMs = 1_786_752_000_000;

class MemorySessionRepository implements SessionRepository {
  private devices = new Map<string, KnownDeviceRecord>();
  private cursors = new Map<string, JournalCursor>();
  private sessions = new Map<string, StoredSessionRecord>();
  private commitCount = 0;
  private failCommitNumber: number | null = null;

  async rememberDevice(device: KnownDeviceRecord): Promise<void> {
    this.devices.set(device.deviceId, device);
  }

  async loadActiveCursor(deviceId: string): Promise<JournalCursor | null> {
    return this.cursors.get(deviceId) ?? null;
  }

  async commitPage(commit: PageCommit): Promise<JournalCursor> {
    const nextSessions = new Map(this.sessions);
    const epoch = bytesToHex(commit.page.journalEpoch);
    for (const record of commit.page.records) {
      nextSessions.set(`${commit.deviceId}:${epoch}:${record.sequence}`, {
        deviceId: commit.deviceId,
        journalEpoch: epoch,
        record,
      });
    }
    const cursor: JournalCursor = {
      journalEpoch: epoch,
      afterSequence: Math.max(
        this.cursors.get(commit.deviceId)?.journalEpoch === epoch
          ? (this.cursors.get(commit.deviceId)?.afterSequence ?? 0)
          : 0,
        nextCursorAfterSequence(commit),
      ),
      completeness: commit.completeness,
      oldestSequence: commit.page.oldestSequence ?? null,
      latestSequence: commit.page.latestSequence ?? null,
    };

    this.commitCount += 1;
    if (this.failCommitNumber === this.commitCount) {
      this.failCommitNumber = null;
      throw new Error('Injected atomic commit failure');
    }

    this.sessions = nextSessions;
    this.cursors.set(commit.deviceId, cursor);
    return cursor;
  }

  async listSessions(deviceId: string): Promise<readonly StoredSessionRecord[]> {
    return [...this.sessions.values()]
      .filter((session) => session.deviceId === deviceId)
      .sort((left, right) => left.record.sequence - right.record.sequence);
  }

  seedCursor(deviceId: string, cursor: JournalCursor): void {
    this.cursors.set(deviceId, cursor);
  }

  failNextCommitAfter(successfulCommits: number): void {
    this.failCommitNumber = this.commitCount + successfulCommits + 1;
  }

  firstRememberedDeviceId(): string | undefined {
    return this.devices.keys().next().value;
  }
}

describe('foreground synchronization against the deterministic protocol mock', () => {
  it('handles empty history and recognizes an already-current cursor', async () => {
    const scenario = getMockScenario('empty');
    const repository = new MemorySessionRepository();
    const transport = createProtocolMockTransport(scenario);

    const first = await synchronizeForeground(
      transport,
      scenario.candidate,
      repository,
      nowUtcMs,
      operation,
    );
    const second = await synchronizeForeground(
      transport,
      scenario.candidate,
      repository,
      nowUtcMs,
      operation,
    );

    expect(first).toMatchObject({ pagesCommitted: 1, recordsReceived: 0 });
    expect(second).toMatchObject({ pagesCommitted: 0, recordsReceived: 0 });
    expect(await repository.listSessions(first.deviceId)).toEqual([]);
  });

  it('commits multiple pages and leaves history available after disconnect', async () => {
    const scenario = getMockScenario('paged');
    const repository = new MemorySessionRepository();
    const transport = createProtocolMockTransport(scenario);

    const result = await synchronizeForeground(
      transport,
      scenario.candidate,
      repository,
      nowUtcMs,
      operation,
    );
    await transport.disconnect();
    const sessions = await repository.listSessions(result.deviceId);

    expect(result).toMatchObject({ pagesCommitted: 3, recordsReceived: 5 });
    expect(result.cursor).toMatchObject({ afterSequence: 5, completeness: 'complete' });
    expect(sessions.map(({ record }) => record.sequence)).toEqual([1, 2, 3, 4, 5]);
    expect(sessions[1]?.record).not.toHaveProperty('startedAtUtcMs');
    expect(sessions[1]?.record).not.toHaveProperty('endedAtUtcMs');
  });

  it('upserts duplicate pages without duplicating records or moving the cursor backwards', async () => {
    const scenario = getMockScenario('duplicated-page');
    const repository = new MemorySessionRepository();
    const transport = createProtocolMockTransport(scenario);
    const result = await synchronizeForeground(
      transport,
      scenario.candidate,
      repository,
      nowUtcMs,
      operation,
    );
    const cursorBefore = await repository.loadActiveCursor(result.deviceId);
    const sessions = await repository.listSessions(result.deviceId);
    const firstTwo = sessions.slice(0, 2).map(({ record }) => record);

    await repository.commitPage({
      deviceId: result.deviceId,
      requestedAfterSequence: 0,
      page: {
        journalEpoch: Uint8Array.from(
          (cursorBefore?.journalEpoch.match(/../gu) ?? []).map((byte) => Number.parseInt(byte, 16)),
        ),
        oldestSequence: 1,
        latestSequence: 3,
        gap: false,
        epochChanged: false,
        records: firstTwo,
        continuationAfterSequence: 2,
      },
      completeness: 'complete',
    });

    expect(await repository.listSessions(result.deviceId)).toHaveLength(3);
    expect((await repository.loadActiveCursor(result.deviceId))?.afterSequence).toBe(3);

    // A normal foreground retry uses status to avoid replaying a current journal.
    const retry = await synchronizeForeground(
      transport,
      scenario.candidate,
      repository,
      nowUtcMs,
      operation,
    );
    expect(retry.recordsReceived).toBe(0);
    expect(await repository.listSessions(result.deviceId)).toHaveLength(3);
  });

  it('retries from the last committed page after a link loss', async () => {
    const scenario = getMockScenario('paged');
    const device = new DeterministicMockDevice(scenario);
    const responder = createProtocolResponder(device);
    let pageRequests = 0;
    let disconnected = false;
    const transport = createMockTransport(scenario, (payload, context) => {
      const request = decodeRequest(payload);
      if (request.request.type === 'getSessionPage') {
        pageRequests += 1;
        if (pageRequests === 2 && !disconnected) {
          disconnected = true;
          throw new DeviceTransportError('connection-lost', true, 'Injected mid-sync link loss');
        }
      }
      return responder(payload, context);
    });
    const repository = new MemorySessionRepository();

    await expect(
      synchronizeForeground(transport, scenario.candidate, repository, nowUtcMs, operation),
    ).rejects.toMatchObject({ code: 'connection-lost' });
    const partialDeviceId = repository.firstRememberedDeviceId()!;
    expect((await repository.loadActiveCursor(partialDeviceId))?.afterSequence).toBe(2);

    const retry = await synchronizeForeground(
      transport,
      scenario.candidate,
      repository,
      nowUtcMs,
      operation,
    );
    expect(retry.recordsReceived).toBe(3);
    expect(await repository.listSessions(retry.deviceId)).toHaveLength(5);
  });

  it('does not advance a cursor when the local transaction fails', async () => {
    const scenario = getMockScenario('paged');
    const repository = new MemorySessionRepository();
    repository.failNextCommitAfter(1);
    const transport = createProtocolMockTransport(scenario);

    await expect(
      synchronizeForeground(transport, scenario.candidate, repository, nowUtcMs, operation),
    ).rejects.toThrow('Injected atomic commit failure');
    const deviceId = repository.firstRememberedDeviceId()!;
    expect((await repository.loadActiveCursor(deviceId))?.afterSequence).toBe(2);
    expect(await repository.listSessions(deviceId)).toHaveLength(2);

    const retry = await synchronizeForeground(
      transport,
      scenario.candidate,
      repository,
      nowUtcMs,
      operation,
    );
    expect(retry.recordsReceived).toBe(3);
    expect(await repository.listSessions(deviceId)).toHaveLength(5);
  });

  it.each([
    ['gap', 'retention-gap', [5, 6, 7]],
    ['changed-epoch', 'changed-epoch', [1, 2]],
  ] as const)('records %s history explicitly', async (scenarioId, completeness, sequences) => {
    const scenario = getMockScenario(scenarioId);
    const repository = new MemorySessionRepository();
    const probe = await synchronizeForeground(
      createProtocolMockTransport('empty'),
      getMockScenario('empty').candidate,
      repository,
      nowUtcMs,
      operation,
    );
    repository.seedCursor(probe.deviceId, {
      journalEpoch: probe.cursor!.journalEpoch,
      afterSequence: scenario.initialClientCursor.afterSequence,
      completeness: 'complete',
      oldestSequence: null,
      latestSequence: scenario.initialClientCursor.afterSequence || null,
    });

    const result = await synchronizeForeground(
      createProtocolMockTransport(scenario),
      scenario.candidate,
      repository,
      nowUtcMs,
      operation,
    );
    expect(result.cursor?.completeness).toBe(completeness);
    expect(
      (await repository.listSessions(result.deviceId)).map(({ record }) => record.sequence),
    ).toEqual(sequences);
  });

  it('surfaces incompatible protocol versions as a structured client error', async () => {
    const scenario = getMockScenario('incompatible-version');
    await expect(
      synchronizeForeground(
        createProtocolMockTransport(scenario),
        scenario.candidate,
        new MemorySessionRepository(),
        nowUtcMs,
        operation,
      ),
    ).rejects.toBeInstanceOf(DeviceClientError);
  });
});
