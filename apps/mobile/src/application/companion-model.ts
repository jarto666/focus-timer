import type { DeviceConnectionState } from '@focus-timer/device-client';
import type { MockScenarioId } from '@focus-timer/mock-device';

export type DeviceStatusModel = Readonly<{
  presetName: string;
  plannedDurationMs: number;
  remainingDurationMs: number;
  viewState: 'idle' | 'running' | 'paused' | 'completed';
  clockKnown: boolean;
}>;

export type LocalHistoryEntry = Readonly<{
  key: string;
  sequence: number;
  presetName: string;
  plannedDurationMs: number;
  activeDurationMs: number;
  outcome: 'completed' | 'cancelled';
  startedAtUtcMs: number | null;
  endedAtUtcMs: number | null;
}>;

export type HistoryCompleteness = 'complete' | 'retention-gap' | 'changed-epoch';

export type LocalHistoryModel = Readonly<{
  entries: readonly LocalHistoryEntry[];
  completeness: HistoryCompleteness;
  journalEpoch: string | null;
}>;

export const emptyHistory: LocalHistoryModel = {
  entries: [],
  completeness: 'complete',
  journalEpoch: null,
};

export type HistorySyncState =
  | Readonly<{ phase: 'loading' | 'syncing' | 'ready' }>
  | Readonly<{ phase: 'failed'; message: string }>;

export type DevelopmentScenarioOption = Readonly<{
  id: MockScenarioId;
  label: string;
}>;

export type CompanionRuntime = Readonly<{
  connection: DeviceConnectionState;
  status: DeviceStatusModel | null;
  history: LocalHistoryModel;
  historySync: HistorySyncState;
  selectedScenario: MockScenarioId;
  developmentScenarios: readonly DevelopmentScenarioOption[];
  startScan(): Promise<void>;
  connect(transportId: string): Promise<void>;
  disconnect(): Promise<void>;
  selectScenario(scenario: MockScenarioId): void;
}>;
