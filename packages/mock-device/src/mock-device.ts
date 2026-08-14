import { protocolVersion } from '@focus-timer/device-protocol';

import type { MockScenario, MockScenarioId, MockSessionRecord } from './scenarios';
import { getMockScenario } from './scenarios';

export type MockHandshakeResult =
  | Readonly<{
      kind: 'compatible';
      device: MockScenario['device'];
    }>
  | Readonly<{
      kind: 'incompatible';
      supportedMajor: number;
      receivedMajor: number;
    }>;

export type MockDeviceStatus = Readonly<{
  viewState: 'idle';
  selectedPreset: Readonly<{
    id: string;
    name: string;
    durationMs: number;
  }>;
  remainingDurationMs: number;
  journalEpoch: string;
  oldestSequence: number | null;
  latestSequence: number | null;
  clockKnown: boolean;
  journalHealthy: boolean;
}>;

export type MockPageRequest = Readonly<{
  journalEpoch: string | null;
  afterSequence: number;
  limit: number;
}>;

export type MockSessionPage = Readonly<{
  journalEpoch: string;
  oldestSequence: number | null;
  latestSequence: number | null;
  gap: boolean;
  records: readonly MockSessionRecord[];
  continuationAfterSequence: number | null;
}>;

export const malformedMockResponseBytes = [0xff, 0x00, 0xd9, 0x7f] as const;

export type MockPageResult =
  | Readonly<{
      kind: 'page';
      page: MockSessionPage;
    }>
  | Readonly<{
      kind: 'changed-epoch';
      requestedEpoch: string;
      page: MockSessionPage;
    }>
  | Readonly<{
      kind: 'malformed-response';
      bytes: Uint8Array;
    }>;

export class DeterministicMockDevice {
  readonly scenario: MockScenario;

  constructor(scenario: MockScenario | MockScenarioId) {
    this.scenario = typeof scenario === 'string' ? getMockScenario(scenario) : scenario;
  }

  handshake(): MockHandshakeResult {
    const receivedMajor = this.scenario.device.protocolVersion.major;

    if (receivedMajor !== protocolVersion.major) {
      return {
        kind: 'incompatible',
        supportedMajor: protocolVersion.major,
        receivedMajor,
      };
    }

    return { kind: 'compatible', device: this.scenario.device };
  }

  readStatus(): MockDeviceStatus {
    const { epoch, records } = this.scenario.journal;
    const oldestSequence = records.at(0)?.sequence ?? null;
    const latestSequence = records.at(-1)?.sequence ?? null;

    return {
      viewState: 'idle',
      selectedPreset: { id: 'pomodoro', name: 'Pomodoro', durationMs: 1_500_000 },
      remainingDurationMs: 1_500_000,
      journalEpoch: epoch,
      oldestSequence,
      latestSequence,
      clockKnown: records.some((record) => record.endedAtUtcMs !== null),
      journalHealthy: true,
    };
  }

  readSessionPage(request: MockPageRequest): MockPageResult {
    if (!Number.isInteger(request.limit) || request.limit < 1) {
      throw new RangeError('Mock page limit must be a positive integer');
    }

    if (!Number.isSafeInteger(request.afterSequence) || request.afterSequence < 0) {
      throw new RangeError('Mock page cursor must be a non-negative safe integer');
    }

    if (this.scenario.transportBehavior === 'malformed-response') {
      return {
        kind: 'malformed-response',
        bytes: Uint8Array.from(malformedMockResponseBytes),
      };
    }

    const page = this.createPage(request.afterSequence, request.limit);

    if (request.journalEpoch !== null && request.journalEpoch !== page.journalEpoch) {
      return {
        kind: 'changed-epoch',
        requestedEpoch: request.journalEpoch,
        page: this.createPage(0, request.limit),
      };
    }

    return { kind: 'page', page };
  }

  private createPage(afterSequence: number, requestedLimit: number): MockSessionPage {
    const { epoch, pageSize, records } = this.scenario.journal;
    const oldestSequence = records.at(0)?.sequence ?? null;
    const latestSequence = records.at(-1)?.sequence ?? null;
    const gap = oldestSequence !== null && afterSequence < oldestSequence - 1;
    const effectiveCursor = gap ? oldestSequence - 1 : afterSequence;
    const effectiveLimit = Math.min(pageSize, requestedLimit);
    const pageRecords = records
      .filter((record) => record.sequence > effectiveCursor)
      .slice(0, effectiveLimit);
    const finalSequence = pageRecords.at(-1)?.sequence ?? null;
    const continuationAfterSequence =
      finalSequence !== null && latestSequence !== null && finalSequence < latestSequence
        ? finalSequence
        : null;

    return {
      journalEpoch: epoch,
      oldestSequence,
      latestSequence,
      gap,
      records: pageRecords,
      continuationAfterSequence,
    };
  }
}

export function createMockDevice(scenario: MockScenario | MockScenarioId): DeterministicMockDevice {
  return new DeterministicMockDevice(scenario);
}
