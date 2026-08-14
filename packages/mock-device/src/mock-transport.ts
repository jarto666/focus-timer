import {
  DeviceTransportError,
  type DeviceCandidate,
  type DeviceTransport,
  type DeviceTransportAvailability,
  type DeviceTransportDisconnect,
  type DeviceTransportOperation,
} from '@focus-timer/device-client';

import { malformedMockResponseBytes } from './mock-device';
import type { MockScenario, MockScenarioId } from './scenarios';
import { getMockScenario } from './scenarios';

export type MockRequestResponder = (
  payload: Uint8Array,
  context: Readonly<{
    scenario: MockScenario;
    connectionNumber: number;
    requestNumber: number;
  }>,
) => Promise<Uint8Array> | Uint8Array;

const echoResponder: MockRequestResponder = (payload) => payload.slice();

export class MockDeviceTransport implements DeviceTransport {
  readonly kind = 'mock';
  readonly scenario: MockScenario;

  private connected = false;
  private connectionNumber = 0;
  private requestNumber = 0;
  private readonly availabilityListeners = new Set<(state: DeviceTransportAvailability) => void>();
  private readonly disconnectListeners = new Set<(event: DeviceTransportDisconnect) => void>();
  private eventListener: ((payload: Uint8Array) => void) | null = null;
  private eventErrorListener: ((error: DeviceTransportError) => void) | null = null;

  constructor(
    scenario: MockScenario | MockScenarioId,
    private readonly responder: MockRequestResponder = echoResponder,
  ) {
    this.scenario = typeof scenario === 'string' ? getMockScenario(scenario) : scenario;
  }

  async readAvailability() {
    return this.scenario.availability;
  }

  async scan(operation: DeviceTransportOperation) {
    this.assertOperation(operation, 'scan-failed');
    this.assertAvailable();
    return [this.scenario.candidate];
  }

  async connect(candidate: DeviceCandidate, operation: DeviceTransportOperation) {
    this.assertOperation(operation, 'connect-failed');
    this.assertAvailable();

    if (candidate.transportId !== this.scenario.candidate.transportId) {
      throw new DeviceTransportError('connect-failed', true, 'Mock candidate is unavailable');
    }

    this.connected = true;
    this.connectionNumber += 1;
    this.requestNumber = 0;
  }

  async disconnect() {
    this.connected = false;
    this.eventListener = null;
    this.eventErrorListener = null;
  }

  async request(payload: Uint8Array, operation: DeviceTransportOperation) {
    this.assertOperation(operation, 'request-timeout');

    if (!this.connected) {
      throw new DeviceTransportError('not-connected', true, 'Connect before requesting');
    }

    const requestNumber = this.requestNumber;
    this.requestNumber += 1;

    if (
      this.scenario.transportBehavior === 'disconnect-on-first-request' &&
      this.connectionNumber === 1 &&
      requestNumber === 0
    ) {
      this.simulateLinkLoss('Deterministic mock disconnect');
      throw new DeviceTransportError('connection-lost', true, 'Mock link was disconnected');
    }

    if (this.scenario.transportBehavior === 'malformed-response') {
      return Uint8Array.from(malformedMockResponseBytes);
    }

    const response = await this.responder(payload.slice(), {
      scenario: this.scenario,
      connectionNumber: this.connectionNumber,
      requestNumber,
    });

    return response.slice();
  }

  subscribeToEvents(
    listener: (payload: Uint8Array) => void,
    onError: (error: DeviceTransportError) => void,
  ) {
    if (!this.connected) {
      throw new DeviceTransportError('not-connected', true, 'Connect before subscribing');
    }
    this.eventListener = listener;
    this.eventErrorListener = onError;
    return () => {
      if (this.eventListener === listener) this.eventListener = null;
      if (this.eventErrorListener === onError) this.eventErrorListener = null;
    };
  }

  emitEvent(payload: Uint8Array) {
    if (this.connected) this.eventListener?.(payload.slice());
  }

  emitEventError(message = 'Mock event stream failed') {
    this.eventErrorListener?.(new DeviceTransportError('transport-failed', true, message));
  }

  subscribeToDisconnect(listener: (event: DeviceTransportDisconnect) => void) {
    this.disconnectListeners.add(listener);
    return () => {
      this.disconnectListeners.delete(listener);
    };
  }

  subscribeToAvailability(listener: (state: DeviceTransportAvailability) => void) {
    this.availabilityListeners.add(listener);
    listener(this.scenario.availability);
    return () => {
      this.availabilityListeners.delete(listener);
    };
  }

  simulateLinkLoss(message: string | null = null) {
    if (!this.connected) {
      return;
    }

    this.connected = false;
    this.eventListener = null;
    this.eventErrorListener = null;
    const event: DeviceTransportDisconnect = { reason: 'link-loss', message };
    for (const listener of this.disconnectListeners) {
      listener(event);
    }
  }

  private assertAvailable() {
    const { availability } = this.scenario;

    if (availability.status === 'unavailable') {
      throw new DeviceTransportError('unavailable', true, 'Mock device is unavailable');
    }

    if (availability.status === 'permission-denied') {
      throw new DeviceTransportError('permission-denied', false, 'Mock permission is denied');
    }
  }

  private assertOperation(
    operation: DeviceTransportOperation,
    timeoutCode: 'scan-failed' | 'connect-failed' | 'request-timeout',
  ) {
    if (operation.cancellation?.cancelled === true) {
      throw new DeviceTransportError('cancelled', false, 'Mock operation was cancelled');
    }

    if (!Number.isFinite(operation.timeoutMs) || operation.timeoutMs <= 0) {
      throw new DeviceTransportError(timeoutCode, true, 'Mock operation timed out');
    }
  }
}

export function createMockTransport(
  scenario: MockScenario | MockScenarioId,
  responder?: MockRequestResponder,
): MockDeviceTransport {
  return responder === undefined
    ? new MockDeviceTransport(scenario)
    : new MockDeviceTransport(scenario, responder);
}
