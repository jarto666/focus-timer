import type {
  ProtocolVersion,
  SessionPageResponse,
  SessionRecord,
} from '@focus-timer/device-protocol';

export type HistoryCompleteness = 'complete' | 'retention-gap' | 'changed-epoch';

export type KnownDeviceRecord = Readonly<{
  deviceId: string;
  transportId: string;
  productName: string;
  firmwareVersion: string;
  protocolVersion: ProtocolVersion;
}>;

export type JournalCursor = Readonly<{
  journalEpoch: string;
  afterSequence: number;
  completeness: HistoryCompleteness;
  oldestSequence: number | null;
  latestSequence: number | null;
}>;

export type StoredSessionRecord = Readonly<{
  deviceId: string;
  journalEpoch: string;
  record: SessionRecord;
}>;

export type PageCommit = Readonly<{
  deviceId: string;
  requestedAfterSequence: number;
  page: SessionPageResponse;
  completeness: HistoryCompleteness;
}>;

/** Persistence boundary required by the foreground catch-up state machine. */
export interface SessionRepository {
  rememberDevice(device: KnownDeviceRecord): Promise<void>;
  loadActiveCursor(deviceId: string): Promise<JournalCursor | null>;
  commitPage(commit: PageCommit): Promise<JournalCursor>;
  listSessions(deviceId: string): Promise<readonly StoredSessionRecord[]>;
}

export function bytesToHex(bytes: Uint8Array): string {
  return [...bytes].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

export function hexToBytes(hex: string): Uint8Array {
  if (!/^(?:[0-9a-f]{2})+$/u.test(hex)) {
    throw new TypeError('Expected non-empty lowercase hexadecimal bytes');
  }
  return Uint8Array.from({ length: hex.length / 2 }, (_, index) =>
    Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16),
  );
}

/** Cursor to persist after a page has been committed successfully. */
export function nextCursorAfterSequence(commit: PageCommit): number {
  const finalRecordSequence = commit.page.records.at(-1)?.sequence;
  if (finalRecordSequence !== undefined) return finalRecordSequence;

  if (commit.page.epochChanged || commit.page.gap) {
    return commit.page.latestSequence ?? 0;
  }

  return commit.page.latestSequence === undefined
    ? commit.requestedAfterSequence
    : Math.min(commit.requestedAfterSequence, commit.page.latestSequence);
}
