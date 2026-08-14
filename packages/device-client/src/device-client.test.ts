import { describe, expect, it } from 'vitest';

import {
  Capability,
  ProtocolErrorCode,
  PROTOCOL_MAJOR,
  PROTOCOL_MINOR,
  decodeRequest,
  encodeEvent,
  encodeResponse,
  type RequestEnvelope,
  type Response,
} from '@focus-timer/device-protocol';

import { DeviceClient, DeviceClientError } from './device-client';
import type {
  DeviceCandidate,
  DeviceTransport,
  DeviceTransportOperation,
} from './device-transport';

const operation = { timeoutMs: 1_000 } as const;
const candidate: DeviceCandidate = {
  transportId: 'test',
  productName: 'FocusTimer',
  rssi: -40,
};

function bytes(length: number, value: number): Uint8Array {
  return Uint8Array.from({ length }, () => value);
}

function helloResponse(): Response {
  return {
    type: 'hello',
    hello: {
      deviceId: bytes(16, 0x11),
      productName: 'FocusTimer',
      firmwareVersion: '0.1.0',
      supportedVersion: { major: PROTOCOL_MAJOR, minor: PROTOCOL_MINOR },
      capabilities: [Capability.ReadStatus, Capability.ReadSessionPages, Capability.SetClockAnchor],
    },
  };
}

class ProtocolTransport implements DeviceTransport {
  readonly kind = 'protocol-test';
  readonly requests: RequestEnvelope[] = [];
  private eventListener: ((payload: Uint8Array) => void) | null = null;

  constructor(private readonly respond: (request: RequestEnvelope) => Response) {}

  async readAvailability() {
    return { status: 'available' } as const;
  }

  async scan() {
    return [candidate];
  }

  async connect() {}

  async disconnect() {}

  async request(payload: Uint8Array, _operation: DeviceTransportOperation) {
    const request = decodeRequest(payload);
    this.requests.push(request);
    return encodeResponse({
      version: request.version,
      requestId: request.requestId,
      response: this.respond(request),
    });
  }

  subscribeToEvents(listener: (payload: Uint8Array) => void) {
    this.eventListener = listener;
    return () => {
      this.eventListener = null;
    };
  }

  emitEvent(payload: Uint8Array) {
    this.eventListener?.(payload);
  }

  subscribeToDisconnect() {
    return () => undefined;
  }

  subscribeToAvailability(listener: (state: { status: 'available' }) => void) {
    listener({ status: 'available' });
    return () => undefined;
  }
}

describe('DeviceClient', () => {
  it('handshakes then exposes correlated typed APIs', async () => {
    const transport = new ProtocolTransport((request) => {
      switch (request.request.type) {
        case 'hello':
          return helloResponse();
        case 'getStatus':
          return {
            type: 'status',
            status: {
              viewState: 0,
              preset: { id: 'focus', name: 'Focus', plannedDurationMs: 3_000_000 },
              remainingDurationMs: 3_000_000,
              journal: { epoch: bytes(8, 0x22), health: 0 },
              clockKnown: false,
            },
          };
        case 'setClockAnchor':
          return {
            type: 'clockAnchor',
            anchor: {
              acceptedUtcMs: request.request.anchor.utcMs,
              deviceMonotonicMsAtReceipt: 42,
            },
          };
        default:
          throw new Error(`Unhandled ${request.request.type}`);
      }
    });
    const client = new DeviceClient(transport);

    await expect(client.getStatus(operation)).rejects.toMatchObject({ code: 'not-handshaken' });
    const hello = await client.handshake(operation);
    expect([...hello.deviceId]).toEqual([...bytes(16, 0x11)]);
    await expect(client.getStatus(operation)).resolves.toMatchObject({
      remainingDurationMs: 3_000_000,
    });
    await expect(client.setClockAnchor(1_800_000_000_000, operation)).resolves.toMatchObject({
      acceptedUtcMs: 1_800_000_000_000,
    });
    expect(transport.requests.map((request) => request.requestId)).toEqual([1, 2, 3]);
  });

  it('reports a structured incompatible handshake', async () => {
    const transport = new ProtocolTransport(() => ({
      type: 'error',
      error: {
        code: ProtocolErrorCode.UnsupportedProtocolVersion,
        supportedVersion: { major: 2, minor: 0 },
      },
    }));
    const client = new DeviceClient(transport);

    await expect(client.handshake(operation)).rejects.toMatchObject({
      name: 'DeviceClientError',
      code: 'incompatible',
      details: { supportedMajor: 2, receivedMajor: 1 },
    });
  });

  it('rejects correlation errors and releases the one-request gate', async () => {
    let mismatched = true;
    const transport = new ProtocolTransport(() => helloResponse());
    const original = transport.request.bind(transport);
    transport.request = async (payload, selectedOperation) => {
      const encoded = await original(payload, selectedOperation);
      if (!mismatched) return encoded;
      mismatched = false;
      const request = decodeRequest(payload);
      return encodeResponse({
        version: request.version,
        requestId: request.requestId + 1,
        response: helloResponse(),
      });
    };

    const client = new DeviceClient(transport);
    await expect(client.handshake(operation)).rejects.toBeInstanceOf(DeviceClientError);
    await expect(client.handshake(operation)).resolves.toMatchObject({ productName: 'FocusTimer' });
  });

  it('orders live events, detects gaps, and exposes catalog operations', async () => {
    const transport = new ProtocolTransport((request) => {
      if (request.request.type === 'hello') {
        const response = helloResponse();
        if (response.type !== 'hello') throw new Error('unreachable');
        return {
          ...response,
          hello: {
            ...response.hello,
            capabilities: [
              ...response.hello.capabilities,
              Capability.LiveStatus,
              Capability.ReadPresetCatalog,
              Capability.ProposePresetCatalog,
            ],
          },
        };
      }
      if (request.request.type === 'getPresetCatalog') {
        return {
          type: 'presetCatalog',
          catalog: {
            revision: 3,
            entries: [
              {
                id: 'deep-work',
                name: 'Deep Work',
                plannedDurationMs: 5_400_000,
                builtIn: true,
              },
              {
                id: 'focus',
                name: 'Focus',
                plannedDurationMs: 3_000_000,
                builtIn: true,
              },
              {
                id: 'pomodoro',
                name: 'Pomodoro',
                plannedDurationMs: 1_500_000,
                builtIn: true,
              },
              {
                id: 'reading',
                name: 'Reading',
                plannedDurationMs: 2_700_000,
                builtIn: true,
              },
              {
                id: 'quick-sprint',
                name: 'Quick Sprint',
                plannedDurationMs: 900_000,
                builtIn: true,
              },
            ],
          },
        };
      }
      if (request.request.type === 'proposePresetCatalog') {
        return {
          type: 'proposePresetCatalog',
          proposal: { proposalId: request.request.proposal.proposalId, expiresInMs: 15_000 },
        };
      }
      throw new Error(`Unhandled ${request.request.type}`);
    });
    const client = new DeviceClient(transport);
    await client.handshake(operation);
    const revisions: number[] = [];
    const recovery: string[] = [];
    const unsubscribe = client.subscribeToEvents(
      (event) => {
        if (event.type === 'liveStatus') revisions.push(event.status.statusRevision!);
      },
      (reason) => recovery.push(reason),
    );
    const live = (revision: number) =>
      encodeEvent({
        version: { major: 1, minor: 1 },
        event: {
          type: 'liveStatus',
          status: {
            viewState: 1,
            preset: { id: 'focus', name: 'Focus', plannedDurationMs: 3_000_000 },
            remainingDurationMs: 2_000_000,
            journal: { epoch: bytes(8, 2), health: 0 },
            clockKnown: true,
            statusEpoch: bytes(8, 9),
            statusRevision: revision,
          },
        },
      });
    transport.emitEvent(live(2));
    transport.emitEvent(live(1));
    transport.emitEvent(live(4));
    transport.emitEvent(Uint8Array.of(0xff));
    expect(revisions).toEqual([2, 4]);
    expect(recovery).toEqual(['gap', 'malformed']);
    await expect(client.getPresetCatalog(operation)).resolves.toMatchObject({ revision: 3 });
    await expect(
      client.proposePresetCatalog(
        {
          expectedRevision: 3,
          proposalId: 41,
          customEntries: [{ id: 'flow', name: 'Flow', plannedDurationMs: 1_800_000 }],
        },
        operation,
      ),
    ).resolves.toMatchObject({ proposalId: 41 });
    unsubscribe();
  });
});
