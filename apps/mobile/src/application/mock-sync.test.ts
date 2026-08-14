import { describe, expect, it } from 'vitest';

import { createMockDevice, getMockScenario } from '@focus-timer/mock-device';

import { MockSyncError, synchronizeMockHistory } from './mock-sync';

describe('mock-backed history model', () => {
  it('collects every bounded page and preserves untimestamped records', () => {
    const scenario = getMockScenario('paged');
    const history = synchronizeMockHistory(createMockDevice(scenario), scenario);

    expect(history.entries.map((entry) => entry.sequence)).toEqual([5, 4, 3, 2, 1]);
    expect(history.entries.find((entry) => entry.sequence === 2)).toMatchObject({
      startedAtUtcMs: null,
      endedAtUtcMs: null,
    });
    expect(history.completeness).toBe('complete');
  });

  it('surfaces retention gaps as incomplete history', () => {
    const scenario = getMockScenario('gap');
    const history = synchronizeMockHistory(createMockDevice(scenario), scenario);

    expect(history.completeness).toBe('retention-gap');
    expect(history.entries.map((entry) => entry.sequence)).toEqual([7, 6, 5]);
  });

  it('surfaces a changed journal generation', () => {
    const scenario = getMockScenario('changed-epoch');
    const history = synchronizeMockHistory(createMockDevice(scenario), scenario);

    expect(history.completeness).toBe('changed-epoch');
    expect(history.journalEpoch).toBe('mock-epoch-02');
  });

  it('rejects malformed responses without fabricating history', () => {
    const scenario = getMockScenario('malformed-response');

    expect(() => synchronizeMockHistory(createMockDevice(scenario), scenario)).toThrow(
      MockSyncError,
    );
  });
});
