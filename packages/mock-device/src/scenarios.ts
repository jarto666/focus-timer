import type {
  DeviceCandidate,
  DeviceTransportAvailability,
  ReadyDevice,
} from '@focus-timer/device-client';

export const mockScenarioIds = [
  'empty',
  'paged',
  'duplicated-page',
  'gap',
  'changed-epoch',
  'disconnect',
  'incompatible-version',
  'malformed-response',
  'unavailable-device',
] as const;

export type MockScenarioId = (typeof mockScenarioIds)[number];

export type MockSessionOutcome = 'completed' | 'cancelled';

export type MockSessionRecord = Readonly<{
  sequence: number;
  presetId: string;
  presetName: string;
  plannedDurationMs: number;
  activeDurationMs: number;
  outcome: MockSessionOutcome;
  startedAtUtcMs: number | null;
  endedAtUtcMs: number | null;
}>;

export type MockJournal = Readonly<{
  epoch: string;
  pageSize: number;
  records: readonly MockSessionRecord[];
}>;

export type MockTransportBehavior =
  'respond' | 'disconnect-on-first-request' | 'malformed-response';

export type MockScenario = Readonly<{
  id: MockScenarioId;
  availability: DeviceTransportAvailability;
  candidate: DeviceCandidate;
  device: ReadyDevice;
  journal: MockJournal;
  initialClientCursor: Readonly<{
    journalEpoch: string | null;
    afterSequence: number;
  }>;
  transportBehavior: MockTransportBehavior;
}>;

const candidate: DeviceCandidate = {
  transportId: 'mock-peripheral-01',
  productName: 'Focus Timer',
  rssi: -48,
};

const compatibleDevice: ReadyDevice = {
  deviceId: 'mock-focus-timer-001',
  transportId: candidate.transportId,
  productName: candidate.productName,
  firmwareVersion: '0.1.0-mock',
  protocolVersion: { major: 1, minor: 0 },
};

function sessionRecord(sequence: number): MockSessionRecord {
  const completed = sequence % 3 !== 0;
  const timestamped = sequence % 2 !== 0;
  const startedAtUtcMs = timestamped ? 1_786_669_200_000 + sequence * 3_600_000 : null;

  return {
    sequence,
    presetId: completed ? 'pomodoro' : 'short-focus',
    presetName: completed ? 'Pomodoro' : 'Short focus',
    plannedDurationMs: completed ? 1_500_000 : 600_000,
    activeDurationMs: completed ? 1_500_000 : 412_000,
    outcome: completed ? 'completed' : 'cancelled',
    startedAtUtcMs,
    endedAtUtcMs: startedAtUtcMs === null ? null : startedAtUtcMs + 1_500_000,
  };
}

function records(from: number, to: number): readonly MockSessionRecord[] {
  return Array.from({ length: to - from + 1 }, (_, index) => sessionRecord(from + index));
}

const available = { status: 'available' } as const;

export const mockScenarios = {
  empty: {
    id: 'empty',
    availability: available,
    candidate,
    device: compatibleDevice,
    journal: { epoch: 'mock-epoch-01', pageSize: 2, records: [] },
    initialClientCursor: { journalEpoch: null, afterSequence: 0 },
    transportBehavior: 'respond',
  },
  paged: {
    id: 'paged',
    availability: available,
    candidate,
    device: compatibleDevice,
    journal: { epoch: 'mock-epoch-01', pageSize: 2, records: records(1, 5) },
    initialClientCursor: { journalEpoch: null, afterSequence: 0 },
    transportBehavior: 'respond',
  },
  'duplicated-page': {
    id: 'duplicated-page',
    availability: available,
    candidate,
    device: compatibleDevice,
    journal: { epoch: 'mock-epoch-01', pageSize: 2, records: records(1, 3) },
    initialClientCursor: { journalEpoch: null, afterSequence: 0 },
    transportBehavior: 'respond',
  },
  gap: {
    id: 'gap',
    availability: available,
    candidate,
    device: compatibleDevice,
    journal: { epoch: 'mock-epoch-01', pageSize: 2, records: records(5, 7) },
    initialClientCursor: { journalEpoch: 'mock-epoch-01', afterSequence: 1 },
    transportBehavior: 'respond',
  },
  'changed-epoch': {
    id: 'changed-epoch',
    availability: available,
    candidate,
    device: compatibleDevice,
    journal: { epoch: 'mock-epoch-02', pageSize: 2, records: records(1, 2) },
    initialClientCursor: { journalEpoch: 'mock-epoch-01', afterSequence: 5 },
    transportBehavior: 'respond',
  },
  disconnect: {
    id: 'disconnect',
    availability: available,
    candidate,
    device: compatibleDevice,
    journal: { epoch: 'mock-epoch-01', pageSize: 2, records: records(1, 3) },
    initialClientCursor: { journalEpoch: null, afterSequence: 0 },
    transportBehavior: 'disconnect-on-first-request',
  },
  'incompatible-version': {
    id: 'incompatible-version',
    availability: available,
    candidate,
    device: {
      ...compatibleDevice,
      firmwareVersion: '0.2.0-future-mock',
      protocolVersion: { major: 2, minor: 0 },
    },
    journal: { epoch: 'mock-epoch-01', pageSize: 2, records: records(1, 2) },
    initialClientCursor: { journalEpoch: null, afterSequence: 0 },
    transportBehavior: 'respond',
  },
  'malformed-response': {
    id: 'malformed-response',
    availability: available,
    candidate,
    device: compatibleDevice,
    journal: { epoch: 'mock-epoch-01', pageSize: 2, records: records(1, 2) },
    initialClientCursor: { journalEpoch: null, afterSequence: 0 },
    transportBehavior: 'malformed-response',
  },
  'unavailable-device': {
    id: 'unavailable-device',
    availability: { status: 'unavailable', reason: 'powered-off' },
    candidate,
    device: compatibleDevice,
    journal: { epoch: 'mock-epoch-01', pageSize: 2, records: [] },
    initialClientCursor: { journalEpoch: null, afterSequence: 0 },
    transportBehavior: 'respond',
  },
} as const satisfies Record<MockScenarioId, MockScenario>;

export function getMockScenario(id: MockScenarioId): MockScenario {
  return mockScenarios[id];
}
