import type {
  BleError,
  BleManager,
  Characteristic,
  Device,
  Subscription,
} from 'react-native-ble-plx';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { DeviceTransportError } from '@focus-timer/device-client';
import { BleFragmenter, BleReassembler, parseBleFrame } from '@focus-timer/device-protocol';

import { base64ToBytes, bytesToBase64 } from './base64';
import {
  BleDeviceTransport,
  FOCUS_TIMER_COMMAND_UUID,
  FOCUS_TIMER_RESPONSE_UUID,
  FOCUS_TIMER_SERVICE_UUID,
} from './ble-device-transport';

vi.mock('react-native-ble-plx', () => ({
  BleManager: class {},
  State: {
    Unknown: 'Unknown',
    Resetting: 'Resetting',
    Unsupported: 'Unsupported',
    Unauthorized: 'Unauthorized',
    PoweredOff: 'PoweredOff',
    PoweredOn: 'PoweredOn',
  },
}));

const operation = { timeoutMs: 250 } as const;
const candidate = { transportId: 'ios-peripheral-id', productName: 'FocusTimer', rssi: -42 };

class FakeSubscription implements Subscription {
  removed = false;

  remove(): void {
    this.removed = true;
  }
}

type NotificationListener = (error: BleError | null, characteristic: Characteristic | null) => void;

class FakeDevice {
  readonly id = candidate.transportId;
  readonly name = candidate.productName;
  readonly localName = candidate.productName;
  readonly rssi = candidate.rssi;
  readonly mtu = 23;
  connected = true;
  malformedResponse = false;
  holdResponse = false;
  monitorInstalledBeforeWrite = false;
  writes = 0;
  response = Uint8Array.from({ length: 96 }, (_, index) => 255 - index);

  private monitor: NotificationListener | null = null;
  private readonly request = new BleReassembler();

  async isConnected(): Promise<boolean> {
    return this.connected;
  }

  async discoverAllServicesAndCharacteristics(): Promise<Device> {
    return this as unknown as Device;
  }

  async characteristicsForService(serviceUuid: string): Promise<Characteristic[]> {
    expect(serviceUuid).toBe(FOCUS_TIMER_SERVICE_UUID);
    return [
      {
        uuid: FOCUS_TIMER_COMMAND_UUID,
        isWritableWithResponse: true,
        isNotifiable: false,
        isIndicatable: false,
      },
      {
        uuid: FOCUS_TIMER_RESPONSE_UUID,
        isWritableWithResponse: false,
        isNotifiable: true,
        isIndicatable: false,
      },
    ] as Characteristic[];
  }

  monitorCharacteristicForService(
    serviceUuid: string,
    characteristicUuid: string,
    listener: NotificationListener,
  ): Subscription {
    expect(serviceUuid).toBe(FOCUS_TIMER_SERVICE_UUID);
    expect(characteristicUuid).toBe(FOCUS_TIMER_RESPONSE_UUID);
    this.monitor = listener;
    return new FakeSubscription();
  }

  async writeCharacteristicWithResponseForService(
    serviceUuid: string,
    characteristicUuid: string,
    encodedFrame: string,
  ): Promise<Characteristic> {
    expect(serviceUuid).toBe(FOCUS_TIMER_SERVICE_UUID);
    expect(characteristicUuid).toBe(FOCUS_TIMER_COMMAND_UUID);
    this.monitorInstalledBeforeWrite = this.monitor !== null;
    this.writes += 1;
    const frame = base64ToBytes(encodedFrame);
    const transferId = parseBleFrame(frame).header.transferId;
    const result = this.request.acceptFrame(frame, Date.now());
    if (result.status === 'complete' && !this.holdResponse) {
      if (this.malformedResponse) {
        this.monitor?.(null, { value: bytesToBase64(Uint8Array.of(1, 2, 3)) } as Characteristic);
      } else {
        const fragments = new BleFragmenter(this.response, transferId, 20);
        for (
          let responseFrame = fragments.nextFrame();
          responseFrame !== undefined;
          responseFrame = fragments.nextFrame()
        ) {
          this.monitor?.(null, {
            value: bytesToBase64(responseFrame),
          } as Characteristic);
        }
      }
    }
    return { value: encodedFrame } as Characteristic;
  }
}

class FakeManager {
  readonly device = new FakeDevice();
  scanStopped = false;
  stateValue = 'PoweredOn';
  disconnectListener: ((error: BleError | null, device: Device | null) => void) | null = null;

  async state(): Promise<string> {
    return this.stateValue;
  }

  onStateChange(): Subscription {
    return new FakeSubscription();
  }

  async startDeviceScan(
    serviceUuids: string[],
    _options: null,
    listener: (error: BleError | null, device: Device | null) => void,
  ): Promise<void> {
    expect(serviceUuids).toEqual([FOCUS_TIMER_SERVICE_UUID]);
    listener(null, this.device as unknown as Device);
  }

  async stopDeviceScan(): Promise<void> {
    this.scanStopped = true;
  }

  async connectToDevice(id: string): Promise<Device> {
    expect(id).toBe(candidate.transportId);
    return this.device as unknown as Device;
  }

  async cancelDeviceConnection(): Promise<Device> {
    this.device.connected = false;
    return this.device as unknown as Device;
  }

  onDeviceDisconnected(
    _id: string,
    listener: (error: BleError | null, device: Device | null) => void,
  ): Subscription {
    this.disconnectListener = listener;
    return new FakeSubscription();
  }

  async cancelTransaction(): Promise<void> {}
}

function transportWith(manager: FakeManager): BleDeviceTransport {
  return new BleDeviceTransport(manager as unknown as BleManager);
}

describe('BLE device transport', () => {
  let manager: FakeManager;
  let transport: BleDeviceTransport;

  beforeEach(() => {
    manager = new FakeManager();
    transport = transportWith(manager);
  });

  it('filters discovery to the Focus Timer service and connects by peripheral hint', async () => {
    await expect(transport.readAvailability()).resolves.toEqual({ status: 'available' });
    await expect(transport.scan(operation)).resolves.toEqual([candidate]);
    expect(manager.scanStopped).toBe(true);
    await expect(transport.connect(candidate, operation)).resolves.toBeUndefined();
  });

  it('distinguishes powered-off Bluetooth from denied permission', async () => {
    manager.stateValue = 'PoweredOff';
    await expect(transport.readAvailability()).resolves.toEqual({
      status: 'unavailable',
      reason: 'powered-off',
    });

    manager.stateValue = 'Unauthorized';
    await expect(transport.readAvailability()).resolves.toEqual({
      status: 'permission-denied',
      canOpenSettings: true,
    });
  });

  it('pre-subscribes, fragments both directions, and correlates the transfer id', async () => {
    await transport.connect(candidate, operation);
    const request = Uint8Array.from({ length: 70 }, (_, index) => index);

    await expect(transport.request(request, operation)).resolves.toEqual(manager.device.response);
    expect(manager.device.monitorInstalledBeforeWrite).toBe(true);
    expect(manager.device.writes).toBeGreaterThan(1);
  });

  it('rejects a second in-flight request and times out a partial exchange', async () => {
    await transport.connect(candidate, operation);
    manager.device.holdResponse = true;
    const first = transport.request(Uint8Array.of(1, 2, 3), { timeoutMs: 25 });

    await expect(transport.request(Uint8Array.of(4), operation)).rejects.toMatchObject({
      name: 'DeviceTransportError',
      code: 'transport-failed',
    });
    await expect(first).rejects.toMatchObject({
      name: 'DeviceTransportError',
      code: 'request-timeout',
    });
  });

  it('recovers after a malformed response', async () => {
    await transport.connect(candidate, operation);
    manager.device.malformedResponse = true;
    await expect(transport.request(Uint8Array.of(1), operation)).rejects.toBeInstanceOf(
      DeviceTransportError,
    );

    manager.device.malformedResponse = false;
    await expect(transport.request(Uint8Array.of(2), operation)).resolves.toEqual(
      manager.device.response,
    );
  });

  it('notifies link loss and rejects the active request', async () => {
    await transport.connect(candidate, operation);
    manager.device.holdResponse = true;
    const events: string[] = [];
    transport.subscribeToDisconnect(({ reason }) => events.push(reason));
    const request = transport.request(Uint8Array.of(1), operation);
    await Promise.resolve();
    manager.disconnectListener?.(new Error('link gone') as BleError, null);

    await expect(request).rejects.toMatchObject({ code: 'connection-lost' });
    expect(events).toEqual(['link-loss']);
  });
});
