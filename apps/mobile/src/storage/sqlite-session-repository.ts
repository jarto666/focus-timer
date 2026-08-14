import type { SQLiteDatabase } from 'expo-sqlite';

import {
  bytesToHex,
  nextCursorAfterSequence,
  type JournalCursor,
  type KnownDeviceRecord,
  type PageCommit,
  type SessionRepository,
  type StoredSessionRecord,
} from '@focus-timer/device-client';
import {
  SessionOutcome,
  type PresetSnapshot,
  type SessionRecord,
} from '@focus-timer/device-protocol';

const DATABASE_NAME = 'muninn.sqlite';
const SCHEMA_VERSION = 2;

export type StoredPresetDraft = Readonly<{
  baseRevision: number;
  customEntries: readonly PresetSnapshot[];
}>;

type CursorRow = Readonly<{
  journal_epoch: string;
  after_sequence: number;
  completeness: JournalCursor['completeness'];
  oldest_sequence: number | null;
  latest_sequence: number | null;
}>;

type SessionRow = Readonly<{
  device_id: string;
  journal_epoch: string;
  sequence: number;
  preset_id: string;
  preset_name: string;
  planned_duration_ms: number;
  active_duration_ms: number;
  outcome: number;
  started_at_utc_ms: number | null;
  ended_at_utc_ms: number | null;
}>;

type DeviceRow = Readonly<{
  device_id: string;
  transport_id: string;
  product_name: string;
  firmware_version: string;
  protocol_major: number;
  protocol_minor: number;
}>;

export class SqliteSessionRepository implements SessionRepository {
  constructor(
    private readonly database: SQLiteDatabase,
    private readonly now: () => number = Date.now,
  ) {}

  async rememberDevice(device: KnownDeviceRecord): Promise<void> {
    await this.database.runAsync(
      `INSERT INTO known_devices (
         device_id, transport_id, product_name, firmware_version,
         protocol_major, protocol_minor, updated_at_ms
       ) VALUES (?, ?, ?, ?, ?, ?, ?)
       ON CONFLICT(device_id) DO UPDATE SET
         transport_id = excluded.transport_id,
         product_name = excluded.product_name,
         firmware_version = excluded.firmware_version,
         protocol_major = excluded.protocol_major,
         protocol_minor = excluded.protocol_minor,
         updated_at_ms = excluded.updated_at_ms`,
      device.deviceId,
      device.transportId,
      device.productName,
      device.firmwareVersion,
      device.protocolVersion.major,
      device.protocolVersion.minor,
      this.now(),
    );
  }

  async loadActiveCursor(deviceId: string): Promise<JournalCursor | null> {
    const row = await this.database.getFirstAsync<CursorRow>(
      `SELECT e.journal_epoch, e.after_sequence, e.completeness,
              e.oldest_sequence, e.latest_sequence
         FROM known_devices AS d
         JOIN journal_epochs AS e
           ON e.device_id = d.device_id
          AND e.journal_epoch = d.active_journal_epoch
        WHERE d.device_id = ?`,
      deviceId,
    );
    return row === null ? null : cursorFromRow(row);
  }

  async loadKnownDevice(deviceId: string): Promise<KnownDeviceRecord | null> {
    const row = await this.database.getFirstAsync<DeviceRow>(
      `SELECT device_id, transport_id, product_name, firmware_version,
              protocol_major, protocol_minor
         FROM known_devices
        WHERE device_id = ?`,
      deviceId,
    );
    return row === null ? null : deviceFromRow(row);
  }

  async loadMostRecentDevice(): Promise<KnownDeviceRecord | null> {
    const row = await this.database.getFirstAsync<DeviceRow>(
      `SELECT device_id, transport_id, product_name, firmware_version,
              protocol_major, protocol_minor
         FROM known_devices
        ORDER BY updated_at_ms DESC
        LIMIT 1`,
    );
    return row === null ? null : deviceFromRow(row);
  }

  async commitPage(commit: PageCommit): Promise<JournalCursor> {
    const journalEpoch = bytesToHex(commit.page.journalEpoch);
    const updatedAtMs = this.now();
    let committedCursor: JournalCursor | null = null;

    await this.database.withExclusiveTransactionAsync(async (transaction) => {
      const existing = await transaction.getFirstAsync<CursorRow>(
        `SELECT journal_epoch, after_sequence, completeness,
                oldest_sequence, latest_sequence
           FROM journal_epochs
          WHERE device_id = ? AND journal_epoch = ?`,
        commit.deviceId,
        journalEpoch,
      );
      const candidateAfterSequence = nextCursorAfterSequence(commit);
      const afterSequence = Math.max(existing?.after_sequence ?? 0, candidateAfterSequence);
      const completeness = preserveIncomplete(existing?.completeness, commit.completeness);

      for (const record of commit.page.records) {
        await upsertSession(transaction, commit.deviceId, journalEpoch, record, updatedAtMs);
      }

      await transaction.runAsync(
        `INSERT INTO journal_epochs (
           device_id, journal_epoch, after_sequence, completeness,
           oldest_sequence, latest_sequence, gap_detected, updated_at_ms
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(device_id, journal_epoch) DO UPDATE SET
           after_sequence = MAX(journal_epochs.after_sequence, excluded.after_sequence),
           completeness = CASE
             WHEN journal_epochs.completeness != 'complete'
               THEN journal_epochs.completeness
             ELSE excluded.completeness
           END,
           oldest_sequence = excluded.oldest_sequence,
           latest_sequence = excluded.latest_sequence,
           gap_detected = MAX(journal_epochs.gap_detected, excluded.gap_detected),
           updated_at_ms = excluded.updated_at_ms`,
        commit.deviceId,
        journalEpoch,
        afterSequence,
        completeness,
        commit.page.oldestSequence ?? null,
        commit.page.latestSequence ?? null,
        completeness === 'complete' ? 0 : 1,
        updatedAtMs,
      );
      await transaction.runAsync(
        `UPDATE known_devices
            SET active_journal_epoch = ?, updated_at_ms = ?
          WHERE device_id = ?`,
        journalEpoch,
        updatedAtMs,
        commit.deviceId,
      );

      committedCursor = {
        journalEpoch,
        afterSequence,
        completeness,
        oldestSequence: commit.page.oldestSequence ?? null,
        latestSequence: commit.page.latestSequence ?? null,
      };
    });

    if (committedCursor === null) {
      throw new Error('SQLite page transaction completed without a cursor');
    }
    return committedCursor;
  }

  async listSessions(deviceId: string): Promise<readonly StoredSessionRecord[]> {
    const rows = await this.database.getAllAsync<SessionRow>(
      `SELECT device_id, journal_epoch, sequence, preset_id, preset_name,
              planned_duration_ms, active_duration_ms, outcome,
              started_at_utc_ms, ended_at_utc_ms
         FROM session_records
        WHERE device_id = ?
        ORDER BY COALESCE(ended_at_utc_ms, started_at_utc_ms, committed_at_ms) DESC,
                 journal_epoch DESC, sequence DESC`,
      deviceId,
    );
    return rows.map(storedSessionFromRow);
  }

  async loadPresetDraft(deviceId: string): Promise<StoredPresetDraft | null> {
    const row = await this.database.getFirstAsync<{
      base_revision: number;
      custom_json: string;
    }>(`SELECT base_revision, custom_json FROM preset_drafts WHERE device_id = ?`, deviceId);
    if (row === null) return null;
    const parsed: unknown = JSON.parse(row.custom_json);
    if (!Array.isArray(parsed)) throw new Error('Stored preset draft is malformed');
    return { baseRevision: row.base_revision, customEntries: parsed as PresetSnapshot[] };
  }

  async savePresetDraft(
    deviceId: string,
    baseRevision: number,
    customEntries: readonly PresetSnapshot[],
  ): Promise<void> {
    await this.database.runAsync(
      `INSERT INTO preset_drafts (device_id, base_revision, custom_json, updated_at_ms)
       VALUES (?, ?, ?, ?)
       ON CONFLICT(device_id) DO UPDATE SET
         base_revision = excluded.base_revision,
         custom_json = excluded.custom_json,
         updated_at_ms = excluded.updated_at_ms`,
      deviceId,
      baseRevision,
      JSON.stringify(customEntries),
      this.now(),
    );
  }

  async clearPresetDraft(deviceId: string): Promise<void> {
    await this.database.runAsync(`DELETE FROM preset_drafts WHERE device_id = ?`, deviceId);
  }
}

let repositoryPromise: Promise<SqliteSessionRepository> | null = null;

export function openSessionRepository(): Promise<SqliteSessionRepository> {
  repositoryPromise ??= import('expo-sqlite').then(async ({ openDatabaseAsync }) => {
    const database = await openDatabaseAsync(DATABASE_NAME);
    await migrate(database);
    return new SqliteSessionRepository(database);
  });
  return repositoryPromise;
}

export async function migrate(database: SQLiteDatabase): Promise<void> {
  await database.execAsync('PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;');
  const version = await database.getFirstAsync<{ user_version: number }>('PRAGMA user_version');
  if ((version?.user_version ?? 0) > SCHEMA_VERSION) {
    throw new Error(`Muninn database schema ${version!.user_version} is newer than this app`);
  }
  if ((version?.user_version ?? 0) === SCHEMA_VERSION) return;

  const currentVersion = version?.user_version ?? 0;
  await database.withExclusiveTransactionAsync(async (transaction) => {
    if (currentVersion < 1) {
      await transaction.execAsync(`
      CREATE TABLE IF NOT EXISTS known_devices (
        device_id TEXT PRIMARY KEY NOT NULL,
        transport_id TEXT NOT NULL,
        product_name TEXT NOT NULL,
        firmware_version TEXT NOT NULL,
        protocol_major INTEGER NOT NULL,
        protocol_minor INTEGER NOT NULL,
        active_journal_epoch TEXT,
        updated_at_ms INTEGER NOT NULL
      );

      CREATE TABLE IF NOT EXISTS journal_epochs (
        device_id TEXT NOT NULL,
        journal_epoch TEXT NOT NULL,
        after_sequence INTEGER NOT NULL CHECK(after_sequence >= 0),
        completeness TEXT NOT NULL
          CHECK(completeness IN ('complete', 'retention-gap', 'changed-epoch')),
        oldest_sequence INTEGER,
        latest_sequence INTEGER,
        gap_detected INTEGER NOT NULL CHECK(gap_detected IN (0, 1)),
        updated_at_ms INTEGER NOT NULL,
        PRIMARY KEY (device_id, journal_epoch),
        FOREIGN KEY (device_id) REFERENCES known_devices(device_id) ON DELETE CASCADE
      );

      CREATE TABLE IF NOT EXISTS session_records (
        device_id TEXT NOT NULL,
        journal_epoch TEXT NOT NULL,
        sequence INTEGER NOT NULL CHECK(sequence > 0),
        preset_id TEXT NOT NULL,
        preset_name TEXT NOT NULL,
        planned_duration_ms INTEGER NOT NULL CHECK(planned_duration_ms > 0),
        active_duration_ms INTEGER NOT NULL CHECK(active_duration_ms >= 0),
        outcome INTEGER NOT NULL CHECK(outcome IN (0, 1)),
        started_at_utc_ms INTEGER,
        ended_at_utc_ms INTEGER,
        committed_at_ms INTEGER NOT NULL,
        PRIMARY KEY (device_id, journal_epoch, sequence),
        FOREIGN KEY (device_id, journal_epoch)
          REFERENCES journal_epochs(device_id, journal_epoch) ON DELETE CASCADE
          DEFERRABLE INITIALLY DEFERRED
      );

      CREATE INDEX IF NOT EXISTS session_records_history
        ON session_records(device_id, ended_at_utc_ms DESC, sequence DESC);

    `);
    }
    if (currentVersion < 2) {
      await transaction.execAsync(`
        CREATE TABLE IF NOT EXISTS preset_drafts (
          device_id TEXT PRIMARY KEY NOT NULL,
          base_revision INTEGER NOT NULL CHECK(base_revision >= 0),
          custom_json TEXT NOT NULL,
          updated_at_ms INTEGER NOT NULL,
          FOREIGN KEY (device_id) REFERENCES known_devices(device_id) ON DELETE CASCADE
        );
      `);
    }
    await transaction.execAsync(`PRAGMA user_version = ${SCHEMA_VERSION};`);
  });
}

async function upsertSession(
  database: SQLiteDatabase,
  deviceId: string,
  journalEpoch: string,
  record: SessionRecord,
  committedAtMs: number,
): Promise<void> {
  await database.runAsync(
    `INSERT INTO session_records (
       device_id, journal_epoch, sequence, preset_id, preset_name,
       planned_duration_ms, active_duration_ms, outcome,
       started_at_utc_ms, ended_at_utc_ms, committed_at_ms
     ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
     ON CONFLICT(device_id, journal_epoch, sequence) DO UPDATE SET
       preset_id = excluded.preset_id,
       preset_name = excluded.preset_name,
       planned_duration_ms = excluded.planned_duration_ms,
       active_duration_ms = excluded.active_duration_ms,
       outcome = excluded.outcome,
       started_at_utc_ms = excluded.started_at_utc_ms,
       ended_at_utc_ms = excluded.ended_at_utc_ms`,
    ...sqlParams(
      deviceId,
      journalEpoch,
      record.sequence,
      record.preset.id,
      record.preset.name,
      record.preset.plannedDurationMs,
      record.activeDurationMs,
      record.outcome,
      record.startedAtUtcMs ?? null,
      record.endedAtUtcMs ?? null,
      committedAtMs,
    ),
  );
}

type SqlValue = string | number | null | Uint8Array;

function sqlParams(...values: SqlValue[]): SqlValue[] {
  return values;
}

function cursorFromRow(row: CursorRow): JournalCursor {
  return {
    journalEpoch: row.journal_epoch,
    afterSequence: row.after_sequence,
    completeness: row.completeness,
    oldestSequence: row.oldest_sequence,
    latestSequence: row.latest_sequence,
  };
}

function deviceFromRow(row: DeviceRow): KnownDeviceRecord {
  return {
    deviceId: row.device_id,
    transportId: row.transport_id,
    productName: row.product_name,
    firmwareVersion: row.firmware_version,
    protocolVersion: { major: row.protocol_major, minor: row.protocol_minor },
  };
}

function preserveIncomplete(
  existing: JournalCursor['completeness'] | undefined,
  incoming: JournalCursor['completeness'],
): JournalCursor['completeness'] {
  return existing === undefined || existing === 'complete' ? incoming : existing;
}

function storedSessionFromRow(row: SessionRow): StoredSessionRecord {
  if (row.outcome !== SessionOutcome.Completed && row.outcome !== SessionOutcome.Cancelled) {
    throw new Error(`Invalid stored session outcome ${row.outcome}`);
  }
  return {
    deviceId: row.device_id,
    journalEpoch: row.journal_epoch,
    record: {
      sequence: row.sequence,
      preset: {
        id: row.preset_id,
        name: row.preset_name,
        plannedDurationMs: row.planned_duration_ms,
      },
      activeDurationMs: row.active_duration_ms,
      outcome: row.outcome,
      ...(row.started_at_utc_ms === null ? {} : { startedAtUtcMs: row.started_at_utc_ms }),
      ...(row.ended_at_utc_ms === null ? {} : { endedAtUtcMs: row.ended_at_utc_ms }),
    },
  };
}
