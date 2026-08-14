import { describe, expect, it } from 'vitest';

import type { DeviceTransportDisconnect } from '@focus-timer/device-client';

import { malformedMockResponseBytes } from './mock-device';
import { createMockTransport } from './mock-transport';

const operation = { timeoutMs: 1_000 } as const;

describe('mock device transport', () => {
  it('discovers and echoes opaque bytes without inventing a wire protocol', async () => {
    const transport = createMockTransport('empty');
    const [candidate] = await transport.scan(operation);

    expect(candidate).toBeDefined();
    if (candidate === undefined) {
      throw new Error('Expected the deterministic mock candidate');
    }

    await transport.connect(candidate, operation);
    const payload = Uint8Array.of(1, 2, 3);
    const response = await transport.request(payload, operation);

    expect(response).toEqual(payload);
    expect(response).not.toBe(payload);
  });

  it('reports an unavailable device before discovery', async () => {
    const transport = createMockTransport('unavailable-device');

    await expect(transport.readAvailability()).resolves.toEqual({
      status: 'unavailable',
      reason: 'powered-off',
    });
    await expect(transport.scan(operation)).rejects.toMatchObject({
      name: 'DeviceTransportError',
      code: 'unavailable',
      retryable: true,
    });
  });

  it('disconnects once, emits link loss, and permits a deterministic retry', async () => {
    const transport = createMockTransport('disconnect');
    const [candidate] = await transport.scan(operation);
    const events: DeviceTransportDisconnect[] = [];
    transport.subscribeToDisconnect((event) => events.push(event));

    if (candidate === undefined) {
      throw new Error('Expected the deterministic mock candidate');
    }

    await transport.connect(candidate, operation);
    await expect(transport.request(Uint8Array.of(7), operation)).rejects.toMatchObject({
      code: 'connection-lost',
      retryable: true,
    });
    expect(events).toEqual([{ reason: 'link-loss', message: 'Deterministic mock disconnect' }]);

    await transport.connect(candidate, operation);
    await expect(transport.request(Uint8Array.of(7), operation)).resolves.toEqual(Uint8Array.of(7));
  });

  it('returns malformed bytes without throwing a transport error', async () => {
    const transport = createMockTransport('malformed-response');
    const [candidate] = await transport.scan(operation);

    if (candidate === undefined) {
      throw new Error('Expected the deterministic mock candidate');
    }

    await transport.connect(candidate, operation);
    const response = await transport.request(Uint8Array.of(1, 2), operation);

    expect([...response]).toEqual(malformedMockResponseBytes);
  });

  it('passes deterministic request counters to a protocol responder', async () => {
    const requestNumbers: number[] = [];
    const transport = createMockTransport('paged', (payload, context) => {
      requestNumbers.push(context.requestNumber);
      return payload;
    });
    const [candidate] = await transport.scan(operation);

    if (candidate === undefined) {
      throw new Error('Expected the deterministic mock candidate');
    }

    await transport.connect(candidate, operation);
    await transport.request(Uint8Array.of(1), operation);
    await transport.request(Uint8Array.of(2), operation);

    expect(requestNumbers).toEqual([0, 1]);
  });
});
