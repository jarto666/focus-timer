import { describe, expect, it } from 'vitest';

import {
  DeviceTransportError,
  type DeviceCandidate,
  type DeviceTransport,
  type DeviceTransportAvailability,
  type DeviceTransportDisconnect,
  type DeviceTransportOperation,
} from './device-transport';

const candidate: DeviceCandidate = {
  transportId: 'mock-transport-1',
  productName: 'Focus Timer',
  rssi: -40,
};

const operation: DeviceTransportOperation = { timeoutMs: 1_000 };

const availabilityStates = [
  { status: 'available' },
  { status: 'unavailable', reason: 'powered-off' },
  { status: 'unavailable', reason: 'unsupported' },
  { status: 'permission-denied', canOpenSettings: true },
] as const satisfies readonly DeviceTransportAvailability[];

class EchoTransport implements DeviceTransport {
  readonly kind = 'test-echo';
  private connected = false;
  private readonly availabilityListeners = new Set<(state: DeviceTransportAvailability) => void>();
  private readonly disconnectListeners = new Set<(event: DeviceTransportDisconnect) => void>();

  async readAvailability() {
    return { status: 'available' } as const;
  }

  async scan() {
    return [candidate];
  }

  async connect(selected: DeviceCandidate) {
    if (selected.transportId !== candidate.transportId) {
      throw new DeviceTransportError('connect-failed', true, 'Unknown candidate');
    }

    this.connected = true;
  }

  async disconnect() {
    this.connected = false;
  }

  async request(payload: Uint8Array) {
    if (!this.connected) {
      throw new DeviceTransportError('not-connected', true, 'Connect before requesting');
    }

    return payload.slice();
  }

  subscribeToEvents() {
    return () => undefined;
  }

  subscribeToDisconnect(listener: (event: DeviceTransportDisconnect) => void) {
    this.disconnectListeners.add(listener);
    return () => this.disconnectListeners.delete(listener);
  }

  subscribeToAvailability(listener: (state: DeviceTransportAvailability) => void) {
    this.availabilityListeners.add(listener);
    listener({ status: 'available' });
    return () => this.availabilityListeners.delete(listener);
  }

  changeAvailability(state: DeviceTransportAvailability) {
    for (const listener of this.availabilityListeners) listener(state);
  }

  loseLink() {
    this.connected = false;
    for (const listener of this.disconnectListeners) {
      listener({ reason: 'link-loss', message: null });
    }
  }
}

describe('DeviceTransport contract', () => {
  it('distinguishes availability from denied permission', () => {
    expect(availabilityStates.map((state) => state.status)).toEqual([
      'available',
      'unavailable',
      'unavailable',
      'permission-denied',
    ]);
  });

  it('supports availability, bounded discovery, connection, and byte requests', async () => {
    const transport: DeviceTransport = new EchoTransport();

    await expect(transport.readAvailability()).resolves.toEqual({ status: 'available' });
    await expect(transport.scan(operation)).resolves.toEqual([candidate]);
    await transport.connect(candidate, operation);

    const request = Uint8Array.from([0, 1, 2, 255]);
    const response = await transport.request(request, operation);

    expect(response).toEqual(request);
    expect(response).not.toBe(request);
  });

  it('reports structured transport failures instead of false readiness', async () => {
    const transport: DeviceTransport = new EchoTransport();

    await expect(transport.request(Uint8Array.of(1), operation)).rejects.toMatchObject({
      name: 'DeviceTransportError',
      code: 'not-connected',
      retryable: true,
    });
  });

  it('delivers link loss and supports listener cleanup', () => {
    const transport = new EchoTransport();
    const events: DeviceTransportDisconnect[] = [];
    const unsubscribe = transport.subscribeToDisconnect((event) => events.push(event));

    transport.loseLink();
    unsubscribe();
    transport.loseLink();

    expect(events).toEqual([{ reason: 'link-loss', message: null }]);
  });

  it('delivers adapter availability changes and supports listener cleanup', () => {
    const transport = new EchoTransport();
    const events: DeviceTransportAvailability[] = [];
    const unsubscribe = transport.subscribeToAvailability((state) => events.push(state));

    transport.changeAvailability({ status: 'unavailable', reason: 'powered-off' });
    unsubscribe();
    transport.changeAvailability({ status: 'available' });

    expect(events).toEqual([
      { status: 'available' },
      { status: 'unavailable', reason: 'powered-off' },
    ]);
  });
});
