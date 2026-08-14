import type {
  DeterministicMockDevice,
  MockScenario,
  MockSessionPage,
} from '@focus-timer/mock-device';

import type { HistoryCompleteness, LocalHistoryEntry, LocalHistoryModel } from './companion-model';

export class MockSyncError extends Error {
  override readonly name = 'MockSyncError';
}

function addPage(entries: Map<string, LocalHistoryEntry>, page: MockSessionPage): void {
  for (const record of page.records) {
    const key = `${page.journalEpoch}:${record.sequence}`;
    entries.set(key, {
      key,
      sequence: record.sequence,
      presetName: record.presetName,
      plannedDurationMs: record.plannedDurationMs,
      activeDurationMs: record.activeDurationMs,
      outcome: record.outcome,
      startedAtUtcMs: record.startedAtUtcMs,
      endedAtUtcMs: record.endedAtUtcMs,
    });
  }
}

export function synchronizeMockHistory(
  device: DeterministicMockDevice,
  scenario: MockScenario,
): LocalHistoryModel {
  const entries = new Map<string, LocalHistoryEntry>();
  let completeness: HistoryCompleteness = 'complete';
  let journalEpoch = scenario.initialClientCursor.journalEpoch;
  let afterSequence = scenario.initialClientCursor.afterSequence;

  for (let pageNumber = 0; pageNumber < 64; pageNumber += 1) {
    const result = device.readSessionPage({
      journalEpoch,
      afterSequence,
      limit: 8,
    });

    if (result.kind === 'malformed-response') {
      throw new MockSyncError('The mock device returned a malformed response');
    }

    if (result.kind === 'changed-epoch') {
      completeness = 'changed-epoch';
      journalEpoch = result.page.journalEpoch;
    } else if (result.page.gap) {
      completeness = 'retention-gap';
    }

    addPage(entries, result.page);
    journalEpoch = result.page.journalEpoch;

    if (result.page.continuationAfterSequence === null) {
      return {
        entries: [...entries.values()].sort((left, right) => right.sequence - left.sequence),
        completeness,
        journalEpoch,
      };
    }

    afterSequence = result.page.continuationAfterSequence;
  }

  throw new MockSyncError('The mock device exceeded the bounded page limit');
}
