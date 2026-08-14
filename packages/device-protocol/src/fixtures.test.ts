import { describe, expect, it } from 'vitest';

import fixtureRegistry from '../../../protocol/fixtures/logical-v1.txt?raw';

import {
  Capability,
  CatalogResult,
  JournalHealth,
  ProtocolErrorCode,
  SessionOutcome,
  ViewState,
  decodeRequest,
  decodeResponse,
  decodeEvent,
  encodeEvent,
  encodeRequest,
  encodeResponse,
  type Request,
  type RequestEnvelope,
  type Response,
  type ResponseEnvelope,
  type DeviceEvent,
  type EventEnvelope,
  type SessionRecord,
} from './index';

const protocolV1 = { major: 1, minor: 0 } as const;

const fixture = (name: string, expectedDirection: 'request' | 'response' | 'event'): Uint8Array => {
  const line = fixtureRegistry
    .split('\n')
    .find((candidate) => !candidate.startsWith('#') && candidate.split(/\s+/u)[0] === name);
  if (line === undefined) throw new Error(`missing fixture: ${name}`);
  const [actualName, actualDirection, hex, unexpected] = line.split(/\s+/u);
  expect(actualName).toBe(name);
  expect(actualDirection).toBe(expectedDirection);
  expect(unexpected).toBeUndefined();
  if (hex === undefined || hex.length % 2 !== 0) throw new Error(`invalid fixture hex: ${name}`);
  return Uint8Array.from(
    Array.from({ length: hex.length / 2 }, (_, index) =>
      Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16),
    ),
  );
};

const assertEvent = (name: string, event: DeviceEvent): void => {
  const expected: EventEnvelope = { version: { major: 1, minor: 1 }, event };
  const bytes = fixture(name, 'event');
  expect(encodeEvent(expected)).toEqual(bytes);
  expect(decodeEvent(bytes)).toEqual(expected);
};

const requestEnvelope = (requestId: number, request: Request): RequestEnvelope => ({
  version: protocolV1,
  requestId,
  request,
});

const responseEnvelope = (requestId: number, response: Response): ResponseEnvelope => ({
  version: protocolV1,
  requestId,
  response,
});

const assertRequest = (name: string, expected: RequestEnvelope): void => {
  const bytes = fixture(name, 'request');
  expect(encodeRequest(expected)).toEqual(bytes);
  expect(decodeRequest(bytes)).toEqual(expected);
};

const assertResponse = (name: string, expected: ResponseEnvelope): void => {
  const bytes = fixture(name, 'response');
  expect(encodeResponse(expected)).toEqual(bytes);
  expect(decodeResponse(bytes)).toEqual(expected);
};

const preset = { id: 'pomodoro', name: 'Pomodoro', plannedDurationMs: 1_500_000 };
const edgePreset = {
  id: 'x'.repeat(32),
  name: 'y'.repeat(32),
  plannedDurationMs: 0xffff_ffff,
};

describe('shared protocol 1.0 golden fixtures', () => {
  it('gates every request kind and request edge', () => {
    assertRequest('hello-request-min', requestEnvelope(1, { type: 'hello' }));
    assertRequest('status-request-max-id', requestEnvelope(0xffff_ffff, { type: 'getStatus' }));
    assertRequest(
      'session-page-request-first',
      requestEnvelope(4, { type: 'getSessionPage', page: { afterSequence: 0, limit: 1 } }),
    );
    assertRequest(
      'session-page-request-max-cursor',
      requestEnvelope(5, {
        type: 'getSessionPage',
        page: {
          journalEpoch: new Uint8Array(8).fill(0xcc),
          afterSequence: Number.MAX_SAFE_INTEGER,
          limit: 8,
        },
      }),
    );
    assertRequest(
      'clock-anchor-request-max',
      requestEnvelope(8, {
        type: 'setClockAnchor',
        anchor: { utcMs: Number.MAX_SAFE_INTEGER },
      }),
    );
  });

  it('gates every response kind and response edge', () => {
    assertResponse(
      'hello-response-full',
      responseEnvelope(1, {
        type: 'hello',
        hello: {
          deviceId: Uint8Array.from({ length: 16 }, (_, index) => index),
          productName: 'Focus Timer',
          firmwareVersion: '0.1.0',
          supportedVersion: protocolV1,
          capabilities: [
            Capability.ReadStatus,
            Capability.ReadSessionPages,
            Capability.SetClockAnchor,
          ],
        },
      }),
    );
    assertResponse(
      'status-response-empty-journal',
      responseEnvelope(2, {
        type: 'status',
        status: {
          viewState: ViewState.Completed,
          preset,
          remainingDurationMs: 0,
          journal: { epoch: new Uint8Array(8).fill(0xaa), health: JournalHealth.Healthy },
          clockKnown: false,
        },
      }),
    );
    assertResponse(
      'status-response-max-bounds',
      responseEnvelope(3, {
        type: 'status',
        status: {
          viewState: ViewState.Paused,
          preset: edgePreset,
          remainingDurationMs: 0xffff_ffff,
          journal: {
            epoch: new Uint8Array(8).fill(0xbb),
            oldestSequence: Number.MAX_SAFE_INTEGER - 1,
            latestSequence: Number.MAX_SAFE_INTEGER,
            health: JournalHealth.Degraded,
          },
          clockKnown: true,
        },
      }),
    );
    assertResponse(
      'session-page-response-empty',
      responseEnvelope(6, {
        type: 'sessionPage',
        page: {
          journalEpoch: new Uint8Array(8).fill(0xdd),
          gap: false,
          epochChanged: true,
          records: [],
        },
      }),
    );

    const records: SessionRecord[] = Array.from({ length: 8 }, (_, index) => ({
      sequence: index + 1,
      preset: edgePreset,
      activeDurationMs: 0xffff_ffff,
      outcome: index % 2 === 0 ? SessionOutcome.Completed : SessionOutcome.Cancelled,
      ...(index === 0 ? { startedAtUtcMs: 0, endedAtUtcMs: 1 } : {}),
      ...(index === 1 ? { endedAtUtcMs: Number.MAX_SAFE_INTEGER } : {}),
      ...(index === 2 ? { startedAtUtcMs: Number.MAX_SAFE_INTEGER } : {}),
    }));
    assertResponse(
      'session-page-response-edge',
      responseEnvelope(7, {
        type: 'sessionPage',
        page: {
          journalEpoch: new Uint8Array(8).fill(0xee),
          oldestSequence: 1,
          latestSequence: 9,
          gap: false,
          epochChanged: false,
          records,
          continuationAfterSequence: 8,
        },
      }),
    );
    assertResponse(
      'clock-anchor-response-edges',
      responseEnvelope(8, {
        type: 'clockAnchor',
        anchor: { acceptedUtcMs: 0, deviceMonotonicMsAtReceipt: Number.MAX_SAFE_INTEGER },
      }),
    );
    assertResponse(
      'error-response-version',
      responseEnvelope(9, {
        type: 'error',
        error: {
          code: ProtocolErrorCode.UnsupportedProtocolVersion,
          failedMessageKind: 1,
          supportedVersion: protocolV1,
        },
      }),
    );
    assertResponse(
      'error-response-field',
      responseEnvelope(10, {
        type: 'error',
        error: {
          code: ProtocolErrorCode.InvalidField,
          failedMessageKind: 5,
          fieldId: 2,
        },
      }),
    );
  });
});

describe('shared protocol 1.1 golden fixtures', () => {
  const protocolV1_1 = { major: 1, minor: 1 } as const;

  it('gates catalog requests and staged response', () => {
    const request = {
      version: protocolV1_1,
      requestId: 11,
      request: { type: 'getPresetCatalog' as const },
    };
    const requestBytes = fixture('catalog-request-v11', 'request');
    expect(encodeRequest(request)).toEqual(requestBytes);
    expect(decodeRequest(requestBytes)).toEqual(request);

    const proposal = {
      version: protocolV1_1,
      requestId: 12,
      request: {
        type: 'proposePresetCatalog' as const,
        proposal: {
          expectedRevision: 7,
          proposalId: 99,
          customEntries: [{ id: 'writing', name: 'Writing', plannedDurationMs: 2_700_000 }],
        },
      },
    };
    const proposalBytes = fixture('catalog-proposal-v11', 'request');
    expect(encodeRequest(proposal)).toEqual(proposalBytes);
    expect(decodeRequest(proposalBytes)).toEqual(proposal);

    const response = {
      version: protocolV1_1,
      requestId: 12,
      response: {
        type: 'proposePresetCatalog' as const,
        proposal: { proposalId: 99, expiresInMs: 15_000 },
      },
    };
    const responseBytes = fixture('catalog-staged-v11', 'response');
    expect(encodeResponse(response)).toEqual(responseBytes);
    expect(decodeResponse(responseBytes)).toEqual(response);
  });

  it('gates live status and catalog result events', () => {
    assertEvent('live-status-v11', {
      type: 'liveStatus',
      status: {
        viewState: ViewState.Running,
        preset,
        remainingDurationMs: 1_234_000,
        journal: {
          epoch: new Uint8Array(8).fill(0x33),
          oldestSequence: 5,
          latestSequence: 19,
          health: JournalHealth.Healthy,
        },
        clockKnown: true,
        statusEpoch: new Uint8Array(8).fill(0x55),
        statusRevision: 41,
      },
    });
    assertEvent('catalog-result-v11', {
      type: 'presetCatalogResult',
      result: { proposalId: 99, result: CatalogResult.Committed, catalogRevision: 8 },
    });
  });
});
