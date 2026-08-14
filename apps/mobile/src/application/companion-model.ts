import type { DeviceConnectionState } from '@focus-timer/device-client';
import type { MockScenarioId } from '@focus-timer/mock-device';

export type DeviceStatusModel = Readonly<{
  presetId: string;
  presetName: string;
  plannedDurationMs: number;
  remainingDurationMs: number;
  viewState: 'idle' | 'running' | 'paused' | 'completed';
  clockKnown: boolean;
  observedAtMs: number;
  freshness: 'live' | 'stale';
}>;

export type PresetDraft = Readonly<{
  id: string;
  name: string;
  plannedDurationMs: number;
}>;

export type PresetCatalogModel = Readonly<{
  revision: number;
  baseRevision: number;
  builtIns: readonly PresetDraft[];
  committedCustom: readonly PresetDraft[];
  draft: readonly PresetDraft[];
}>;

export type PresetSyncState =
  | 'unavailable'
  | 'loading'
  | 'synchronized'
  | 'unsynchronized'
  | 'awaiting-confirmation'
  | 'busy'
  | 'conflict'
  | 'expired'
  | 'rejected'
  | 'storage-failed';

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
  presetCatalog: PresetCatalogModel | null;
  presetSync: PresetSyncState;
  selectedScenario: MockScenarioId;
  developmentScenarios: readonly DevelopmentScenarioOption[];
  startScan(): Promise<void>;
  connect(transportId: string): Promise<void>;
  disconnect(): Promise<void>;
  updatePresetDraft(entries: readonly PresetDraft[]): Promise<void>;
  submitPresetDraft(): Promise<void>;
  selectScenario(scenario: MockScenarioId): void;
}>;
