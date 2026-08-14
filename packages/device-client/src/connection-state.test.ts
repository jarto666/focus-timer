import { describe, expect, it } from 'vitest';

import {
  connectionActionsFor,
  connectionPhases,
  isReadyConnection,
  type DeviceConnectionState,
} from './connection-state';
import type { DeviceCandidate } from './device-transport';

const candidate: DeviceCandidate = {
  transportId: 'ios-peripheral-42',
  productName: 'Focus Timer',
  rssi: -54,
};

const readyDevice = {
  deviceId: 'stable-device-123',
  transportId: candidate.transportId,
  productName: candidate.productName,
  firmwareVersion: '0.1.0',
  protocolVersion: { major: 1, minor: 0 },
} as const;

const states = [
  { phase: 'unavailable', reason: 'powered-off' },
  { phase: 'permission-denied', canOpenSettings: true },
  { phase: 'scanning', candidates: [candidate] },
  { phase: 'connecting', candidate },
  { phase: 'handshaking', candidate },
  { phase: 'ready', device: readyDevice },
  { phase: 'disconnected', reason: 'link-loss', lastDevice: readyDevice },
  { phase: 'incompatible', candidate, supportedMajor: 1, receivedMajor: 2 },
  {
    phase: 'retryable-error',
    candidate,
    error: {
      code: 'handshake-timeout',
      operation: 'handshake',
      message: 'Handshake timed out',
    },
  },
] as const satisfies readonly DeviceConnectionState[];

describe('device connection state', () => {
  it('keeps every required phase explicit and distinguishable', () => {
    expect(states.map((state) => state.phase)).toEqual(connectionPhases);
  });

  it('only permits protocol requests after a successful handshake', () => {
    for (const state of states) {
      expect(connectionActionsFor(state).canRequest).toBe(state.phase === 'ready');
      expect(isReadyConnection(state)).toBe(state.phase === 'ready');
    }
  });

  it('exposes recovery without inventing a connected state', () => {
    expect(connectionActionsFor(states[0])).toMatchObject({ canRetry: true });
    expect(connectionActionsFor(states[1])).toMatchObject({ canRetry: true });
    expect(connectionActionsFor(states[6])).toMatchObject({ canScan: true });
    expect(connectionActionsFor(states[7])).toMatchObject({ canScan: true });
    expect(connectionActionsFor(states[8])).toMatchObject({ canRetry: true });
  });

  it('does not offer a meaningless retry on unsupported hardware', () => {
    expect(connectionActionsFor({ phase: 'unavailable', reason: 'unsupported' })).toMatchObject({
      canRetry: false,
      canScan: false,
    });
  });

  it('keeps the stable device identity separate from the transport hint', () => {
    const ready = states[5];

    expect(ready.device.deviceId).toBe('stable-device-123');
    expect(ready.device.transportId).toBe('ios-peripheral-42');
    expect(ready.device.deviceId).not.toBe(ready.device.transportId);
  });
});
