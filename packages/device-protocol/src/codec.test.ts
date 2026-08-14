import { describe, expect, it } from 'vitest';

import {
  Capability,
  CatalogResult,
  JournalHealth,
  MAX_LOGICAL_MESSAGE_BYTES,
  MAX_RECORDS_PER_PAGE,
  ProtocolEncodeError,
  ProtocolErrorCode,
  SessionOutcome,
  ViewState,
  decodeRequest,
  decodeResponse,
  decodeEvent,
  encodeEvent,
  encodeRequest,
  encodeResponse,
  protocolVersion,
  type PresetSnapshot,
  type DeviceEvent,
  type EventEnvelope,
  type DecodeErrorCode,
  type ProtocolDecodeError,
  type Request,
  type RequestEnvelope,
  type Response,
  type ResponseEnvelope,
  type SessionRecord,
} from './index';

const preset = (): PresetSnapshot => ({
  id: 'pomodoro',
  name: 'Pomodoro',
  plannedDurationMs: 1_500_000,
});

const requestEnvelope = (request: Request): RequestEnvelope => ({
  version: protocolVersion,
  requestId: 42,
  request,
});

const responseEnvelope = (response: Response): ResponseEnvelope => ({
  version: protocolVersion,
  requestId: 42,
  response,
});

const roundTripRequest = (request: Request): void => {
  const expected = requestEnvelope(request);
  expect(decodeRequest(encodeRequest(expected))).toEqual(expected);
};

const roundTripResponse = (response: Response): void => {
  const expected = responseEnvelope(response);
  expect(decodeResponse(encodeResponse(expected))).toEqual(expected);
};

const roundTripEvent = (event: DeviceEvent): void => {
  const expected: EventEnvelope = { version: protocolVersion, event };
  expect(decodeEvent(encodeEvent(expected))).toEqual(expected);
};

const catalog = () => ({
  revision: 7,
  entries: [
    { id: 'deep-work', name: 'Deep Work', plannedDurationMs: 5_400_000, builtIn: true },
    { id: 'focus', name: 'Focus', plannedDurationMs: 3_000_000, builtIn: true },
    { id: 'pomodoro', name: 'Pomodoro', plannedDurationMs: 1_500_000, builtIn: true },
    { id: 'reading', name: 'Reading', plannedDurationMs: 2_700_000, builtIn: true },
    { id: 'quick-sprint', name: 'Quick Sprint', plannedDurationMs: 900_000, builtIn: true },
    { id: 'writing', name: 'Writing', plannedDurationMs: 2_700_000, builtIn: false },
  ],
});

describe('canonical logical protocol codec', () => {
  it('round-trips every request message', () => {
    roundTripRequest({ type: 'hello' });
    roundTripRequest({ type: 'getStatus' });
    roundTripRequest({
      type: 'getSessionPage',
      page: { journalEpoch: new Uint8Array(8).fill(0x22), afterSequence: 7, limit: 8 },
    });
    roundTripRequest({
      type: 'getSessionPage',
      page: { afterSequence: 0, limit: 1 },
    });
    roundTripRequest({
      type: 'setClockAnchor',
      anchor: { utcMs: 1_786_669_200_000 },
    });
    roundTripRequest({ type: 'getPresetCatalog' });
    roundTripRequest({
      type: 'proposePresetCatalog',
      proposal: {
        expectedRevision: 7,
        proposalId: 99,
        customEntries: [{ id: 'writing', name: 'Writing', plannedDurationMs: 2_700_000 }],
      },
    });
    roundTripRequest({ type: 'unknown', messageKind: 200 });
  });

  it('round-trips catalogs, proposals, and unsolicited events', () => {
    roundTripResponse({ type: 'presetCatalog', catalog: catalog() });
    roundTripResponse({
      type: 'proposePresetCatalog',
      proposal: { proposalId: 99, expiresInMs: 15_000 },
    });
    roundTripEvent({
      type: 'liveStatus',
      status: {
        viewState: ViewState.Running,
        preset: preset(),
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
    roundTripEvent({
      type: 'presetCatalogResult',
      result: { proposalId: 99, result: CatalogResult.Committed, catalogRevision: 8 },
    });
    roundTripEvent({
      type: 'presetCatalogResult',
      result: { proposalId: 100, result: CatalogResult.Rejected },
    });
  });

  it('round-trips hello, status, page, clock, and structured errors', () => {
    roundTripResponse({
      type: 'hello',
      hello: {
        deviceId: new Uint8Array(16).fill(0x11),
        productName: 'Focus Timer',
        firmwareVersion: '0.1.0',
        supportedVersion: protocolVersion,
        capabilities: [
          Capability.ReadStatus,
          Capability.ReadSessionPages,
          Capability.SetClockAnchor,
        ],
      },
    });
    roundTripResponse({
      type: 'status',
      status: {
        viewState: ViewState.Paused,
        preset: preset(),
        remainingDurationMs: 840_000,
        journal: {
          epoch: new Uint8Array(8).fill(0x33),
          oldestSequence: 5,
          latestSequence: 19,
          health: JournalHealth.Degraded,
        },
        clockKnown: true,
      },
    });
    roundTripResponse({
      type: 'status',
      status: {
        viewState: ViewState.Idle,
        preset: preset(),
        remainingDurationMs: 1_500_000,
        journal: {
          epoch: new Uint8Array(8).fill(0x33),
          health: JournalHealth.Healthy,
        },
        clockKnown: false,
      },
    });
    roundTripResponse({
      type: 'sessionPage',
      page: {
        journalEpoch: new Uint8Array(8).fill(0x44),
        oldestSequence: 5,
        latestSequence: 9,
        gap: true,
        epochChanged: false,
        records: [
          {
            sequence: 5,
            preset: preset(),
            activeDurationMs: 1_500_000,
            outcome: SessionOutcome.Completed,
            startedAtUtcMs: 1_786_669_200_000,
            endedAtUtcMs: 1_786_670_700_000,
          },
          {
            sequence: 6,
            preset: preset(),
            activeDurationMs: 412_000,
            outcome: SessionOutcome.Cancelled,
          },
        ],
        continuationAfterSequence: 6,
      },
    });
    roundTripResponse({
      type: 'clockAnchor',
      anchor: { acceptedUtcMs: 1_786_669_200_000, deviceMonotonicMsAtReceipt: 123_456 },
    });
    roundTripResponse({
      type: 'error',
      error: {
        code: ProtocolErrorCode.UnsupportedProtocolVersion,
        failedMessageKind: 1,
        supportedVersion: protocolVersion,
      },
    });
    roundTripResponse({
      type: 'error',
      error: {
        code: ProtocolErrorCode.InvalidField,
        failedMessageKind: 5,
        fieldId: 2,
      },
    });
  });

  it('emits the registry canonical bytes for hello', () => {
    expect(
      Array.from(
        encodeRequest({
          ...requestEnvelope({ type: 'hello' }),
          version: { major: 1, minor: 0 },
          requestId: 7,
        }),
      ),
    ).toEqual([0xa5, 0x00, 0x01, 0x01, 0x00, 0x02, 0x07, 0x03, 0x01, 0x04, 0xa0]);
  });

  it('keeps the maximum bounded page below 2 KiB', () => {
    const records: SessionRecord[] = Array.from({ length: MAX_RECORDS_PER_PAGE }, (_, index) => ({
      sequence: index + 1,
      preset: { id: 'x'.repeat(32), name: 'x'.repeat(32), plannedDurationMs: 0xffff_ffff },
      activeDurationMs: 0xffff_ffff,
      outcome: SessionOutcome.Completed,
      startedAtUtcMs: Number.MAX_SAFE_INTEGER - 1,
      endedAtUtcMs: Number.MAX_SAFE_INTEGER,
    }));
    const envelope: ResponseEnvelope = {
      version: { major: 1, minor: 0 },
      requestId: 0xffff_ffff,
      response: {
        type: 'sessionPage',
        page: {
          journalEpoch: new Uint8Array(8).fill(0xff),
          oldestSequence: 1,
          latestSequence: 8,
          gap: false,
          epochChanged: false,
          records,
        },
      },
    };
    const encoded = encodeResponse(envelope);
    expect(encoded.length).toBeLessThan(MAX_LOGICAL_MESSAGE_BYTES);
    expect(decodeResponse(encoded)).toEqual(envelope);
  });

  it('rejects invalid models before encoding', () => {
    expect(() =>
      encodeRequest({
        version: protocolVersion,
        requestId: 1,
        request: { type: 'getSessionPage', page: { afterSequence: 1, limit: 8 } },
      }),
    ).toThrowError(expect.objectContaining({ code: 'invalidValue', field: 'journalEpoch' }));

    expect(() =>
      encodeRequest({ ...requestEnvelope({ type: 'hello' }), requestId: 0 }),
    ).toThrowError(ProtocolEncodeError);

    expect(() =>
      encodeRequest(
        requestEnvelope({
          type: 'proposePresetCatalog',
          proposal: {
            expectedRevision: 0,
            proposalId: 1,
            customEntries: [{ id: 'bad', name: 'Bad', plannedDurationMs: 60_001 }],
          },
        }),
      ),
    ).toThrowError(expect.objectContaining({ field: 'plannedDurationMs' }));

    expect(() =>
      encodeEvent({
        version: protocolVersion,
        event: {
          type: 'liveStatus',
          status: {
            viewState: ViewState.Idle,
            preset: preset(),
            remainingDurationMs: 1_500_000,
            journal: {
              epoch: new Uint8Array(8),
              health: JournalHealth.Healthy,
            },
            clockKnown: false,
          },
        },
      }),
    ).toThrowError(expect.objectContaining({ field: 'statusEpoch' }));
  });

  it('rejects non-canonical, duplicate, forbidden, and trailing values', () => {
    const invalid: readonly [readonly number[], DecodeErrorCode][] = [
      [[0xa5, 0, 1, 1, 0, 2, 0x18, 7, 3, 1, 4, 0xa0], 'nonCanonical'],
      [[0xa5, 0, 1, 0, 0, 2, 1, 3, 1, 4, 0xa0], 'duplicateOrUnorderedKey'],
      [[0xa6, 0, 1, 1, 0, 2, 1, 3, 1, 4, 0xa0, 5, 0xf6], 'unsupportedType'],
      [[0xa5, 0, 1, 1, 0, 2, 1, 3, 1, 4, 0xa0, 0], 'trailingData'],
    ];
    for (const [bytes, code] of invalid) {
      expect(() => decodeRequest(Uint8Array.from(bytes))).toThrowError(
        expect.objectContaining<Partial<ProtocolDecodeError>>({ code }),
      );
    }
  });

  it('validates and ignores a compatible unknown field', () => {
    const decoded = decodeRequest(
      Uint8Array.from([0xa6, 0, 1, 1, 0, 2, 1, 3, 1, 4, 0xa0, 5, 0xf5]),
    );
    expect(decoded).toEqual({
      version: { major: 1, minor: 0 },
      requestId: 1,
      request: { type: 'hello' },
    });
  });

  it('bounds empty and oversized input', () => {
    expect(() => decodeRequest(new Uint8Array())).toThrowError(
      expect.objectContaining<Partial<ProtocolDecodeError>>({ code: 'empty' }),
    );
    expect(() => decodeRequest(new Uint8Array(MAX_LOGICAL_MESSAGE_BYTES + 1))).toThrowError(
      expect.objectContaining<Partial<ProtocolDecodeError>>({ code: 'messageTooLarge' }),
    );
  });
});
