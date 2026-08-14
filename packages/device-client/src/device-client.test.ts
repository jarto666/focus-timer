import { describe, expect, it } from 'vitest';

import {
  Capability,
  ProtocolErrorCode,
  PROTOCOL_MAJOR,
  PROTOCOL_MINOR,
  decodeRequest,
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

  subscribeToDisconnect() {
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
});
