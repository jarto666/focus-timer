/// <reference types="node" />

import { DatabaseSync, type SQLInputValue } from 'node:sqlite';

import { describe, expect, it } from 'vitest';

import type { SQLiteDatabase } from 'expo-sqlite';

import type { KnownDeviceRecord, PageCommit } from '@focus-timer/device-client';
import { SessionOutcome } from '@focus-timer/device-protocol';

import { SqliteSessionRepository, migrate } from './sqlite-session-repository';

class NodeSqliteAdapter {
  private readonly database = new DatabaseSync(':memory:');
  private failureNeedle: string | null = null;

  async execAsync(source: string): Promise<void> {
    this.maybeFail(source);
    this.database.exec(source);
  }

  async runAsync(source: string, ...params: SQLInputValue[]) {
    this.maybeFail(source);
    const result = this.database.prepare(source).run(...params);
    return {
      changes: Number(result.changes),
      lastInsertRowId: Number(result.lastInsertRowid),
    };
  }

  async getFirstAsync<T>(source: string, ...params: SQLInputValue[]): Promise<T | null> {
    this.maybeFail(source);
    return (this.database.prepare(source).get(...params) as T | undefined) ?? null;
  }

  async getAllAsync<T>(source: string, ...params: SQLInputValue[]): Promise<T[]> {
    this.maybeFail(source);
    return this.database.prepare(source).all(...params) as T[];
  }

  async withExclusiveTransactionAsync(
    task: (transaction: SQLiteDatabase) => Promise<void>,
  ): Promise<void> {
    this.database.exec('BEGIN IMMEDIATE');
    try {
      await task(this.asExpoDatabase());
      this.database.exec('COMMIT');
    } catch (error) {
      this.database.exec('ROLLBACK');
      throw error;
    }
  }

  failOnceWhenSqlIncludes(needle: string): void {
    this.failureNeedle = needle;
  }

  asExpoDatabase(): SQLiteDatabase {
    return this as unknown as SQLiteDatabase;
  }

  private maybeFail(source: string): void {
    if (this.failureNeedle !== null && source.includes(this.failureNeedle)) {
      this.failureNeedle = null;
      throw new Error('Injected SQLite write failure');
    }
  }
}

const device: KnownDeviceRecord = {
  deviceId: '11111111111111111111111111111111',
  transportId: 'ios-peripheral-hint',
  productName: 'FocusTimer',
  firmwareVersion: '0.1.0',
  protocolVersion: { major: 1, minor: 0 },
};

const epoch = Uint8Array.from({ length: 8 }, () => 0x22);

function page(
  sequences: readonly number[],
  requestedAfterSequence: number,
  continuationAfterSequence?: number,
): PageCommit {
  return {
    deviceId: device.deviceId,
    requestedAfterSequence,
    completeness: 'complete',
    page: {
      journalEpoch: epoch,
      oldestSequence: 1,
      latestSequence: 3,
      gap: false,
      epochChanged: false,
      records: sequences.map((sequence) => ({
        sequence,
        preset: {
          id: 'pomodoro',
          name: 'Pomodoro',
          plannedDurationMs: 1_500_000,
        },
        activeDurationMs: sequence === 3 ? 420_000 : 1_500_000,
        outcome: sequence === 3 ? SessionOutcome.Cancelled : SessionOutcome.Completed,
        ...(sequence === 2
          ? {}
          : {
              startedAtUtcMs: 1_786_669_200_000 + sequence * 2_000_000,
              endedAtUtcMs: 1_786_670_700_000 + sequence * 2_000_000,
            }),
      })),
      ...(continuationAfterSequence === undefined ? {} : { continuationAfterSequence }),
    },
  };
}

describe('SqliteSessionRepository', () => {
  it('migrates idempotently and atomically commits an idempotent page', async () => {
    const adapter = new NodeSqliteAdapter();
    const database = adapter.asExpoDatabase();
    await migrate(database);
    await migrate(database);
    const repository = new SqliteSessionRepository(database, () => 1_800_000_000_000);
    await repository.rememberDevice(device);

    await repository.commitPage(page([1, 2], 0, 2));
    await repository.commitPage(page([1, 2], 0, 2));

    expect(await repository.loadActiveCursor(device.deviceId)).toMatchObject({
      afterSequence: 2,
      completeness: 'complete',
    });
    const sessions = await repository.listSessions(device.deviceId);
    expect(sessions).toHaveLength(2);
    expect(sessions.find(({ record }) => record.sequence === 2)?.record).not.toHaveProperty(
      'startedAtUtcMs',
    );
  });

  it('rolls back records and cursor together, then succeeds on retry', async () => {
    const adapter = new NodeSqliteAdapter();
    const database = adapter.asExpoDatabase();
    await migrate(database);
    const repository = new SqliteSessionRepository(database, () => 1_800_000_000_000);
    await repository.rememberDevice(device);
    await repository.commitPage(page([1, 2], 0, 2));

    adapter.failOnceWhenSqlIncludes('INSERT INTO journal_epochs');
    await expect(repository.commitPage(page([3], 2))).rejects.toThrow(
      'Injected SQLite write failure',
    );
    expect((await repository.loadActiveCursor(device.deviceId))?.afterSequence).toBe(2);
    expect(await repository.listSessions(device.deviceId)).toHaveLength(2);

    await repository.commitPage(page([3], 2));
    expect((await repository.loadActiveCursor(device.deviceId))?.afterSequence).toBe(3);
    expect(await repository.listSessions(device.deviceId)).toHaveLength(3);
  });

  it('preserves incomplete-history metadata across later duplicate pages', async () => {
    const adapter = new NodeSqliteAdapter();
    const database = adapter.asExpoDatabase();
    await migrate(database);
    const repository = new SqliteSessionRepository(database);
    await repository.rememberDevice(device);

    await repository.commitPage({ ...page([1, 2], 0, 2), completeness: 'retention-gap' });
    await repository.commitPage(page([1, 2], 0, 2));

    expect((await repository.loadActiveCursor(device.deviceId))?.completeness).toBe(
      'retention-gap',
    );
  });
});
