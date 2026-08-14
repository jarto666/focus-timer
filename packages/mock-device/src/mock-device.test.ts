import { describe, expect, it } from 'vitest';

import { createMockDevice, malformedMockResponseBytes } from './mock-device';
import { mockScenarioIds } from './scenarios';

describe('deterministic mock device scenarios', () => {
  it('registers every required scenario exactly once', () => {
    expect(mockScenarioIds).toEqual([
      'empty',
      'paged',
      'duplicated-page',
      'gap',
      'changed-epoch',
      'disconnect',
      'incompatible-version',
      'malformed-response',
      'unavailable-device',
    ]);
    expect(new Set(mockScenarioIds).size).toBe(mockScenarioIds.length);
  });

  it('returns an honest empty journal', () => {
    const device = createMockDevice('empty');
    const result = device.readSessionPage({
      journalEpoch: null,
      afterSequence: 0,
      limit: 8,
    });

    expect(result).toEqual({
      kind: 'page',
      page: {
        journalEpoch: 'mock-epoch-01',
        oldestSequence: null,
        latestSequence: null,
        gap: false,
        records: [],
        continuationAfterSequence: null,
      },
    });
  });

  it('serves paged history in stable ascending order', () => {
    const device = createMockDevice('paged');
    const sequences: number[] = [];
    let afterSequence = 0;

    for (;;) {
      const result = device.readSessionPage({
        journalEpoch: 'mock-epoch-01',
        afterSequence,
        limit: 8,
      });

      expect(result.kind).toBe('page');
      if (result.kind !== 'page') {
        throw new Error(`Unexpected mock result: ${result.kind}`);
      }

      sequences.push(...result.page.records.map((record) => record.sequence));
      if (result.page.continuationAfterSequence === null) {
        break;
      }
      afterSequence = result.page.continuationAfterSequence;
    }

    expect(sequences).toEqual([1, 2, 3, 4, 5]);
  });

  it('returns byte-for-byte equivalent logical pages for a repeated cursor', () => {
    const device = createMockDevice('duplicated-page');
    const request = { journalEpoch: 'mock-epoch-01', afterSequence: 0, limit: 2 };

    expect(device.readSessionPage(request)).toEqual(device.readSessionPage(request));
  });

  it('reports a stale cursor gap and starts from the oldest retained record', () => {
    const device = createMockDevice('gap');
    const result = device.readSessionPage({
      journalEpoch: 'mock-epoch-01',
      afterSequence: 1,
      limit: 8,
    });

    expect(result.kind).toBe('page');
    if (result.kind !== 'page') {
      throw new Error(`Unexpected mock result: ${result.kind}`);
    }

    expect(result.page.gap).toBe(true);
    expect(result.page.oldestSequence).toBe(5);
    expect(result.page.records.map((record) => record.sequence)).toEqual([5, 6]);
  });

  it('makes a journal epoch change explicit', () => {
    const device = createMockDevice('changed-epoch');
    const result = device.readSessionPage({
      journalEpoch: 'mock-epoch-01',
      afterSequence: 5,
      limit: 8,
    });

    expect(result).toMatchObject({
      kind: 'changed-epoch',
      requestedEpoch: 'mock-epoch-01',
      page: {
        journalEpoch: 'mock-epoch-02',
        records: [{ sequence: 1 }, { sequence: 2 }],
      },
    });
  });

  it('reports an incompatible protocol major during handshake', () => {
    expect(createMockDevice('incompatible-version').handshake()).toEqual({
      kind: 'incompatible',
      supportedMajor: 1,
      receivedMajor: 2,
    });
  });

  it('produces deterministic malformed response bytes', () => {
    const result = createMockDevice('malformed-response').readSessionPage({
      journalEpoch: null,
      afterSequence: 0,
      limit: 8,
    });

    expect(result.kind).toBe('malformed-response');
    if (result.kind !== 'malformed-response') {
      throw new Error(`Unexpected mock result: ${result.kind}`);
    }
    expect([...result.bytes]).toEqual(malformedMockResponseBytes);
  });
});
