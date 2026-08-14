import {
  BleManager,
  State,
  type BleError,
  type Device,
  type Subscription,
} from 'react-native-ble-plx';

import {
  DeviceTransportError,
  type DeviceCandidate,
  type DeviceTransport,
  type DeviceTransportAvailability,
  type DeviceTransportDisconnect,
  type DeviceTransportErrorCode,
  type DeviceTransportOperation,
} from '@focus-timer/device-client';
import {
  BLE_FRAME_HEADER_BYTES,
  BleFragmenter,
  BleReassembler,
  nextTransferId,
  parseBleFrame,
} from '@focus-timer/device-protocol';

import { base64ToBytes, bytesToBase64 } from './base64';

export const FOCUS_TIMER_SERVICE_UUID = '1cf47046-2e37-4642-a30e-df24879f994f';
export const FOCUS_TIMER_COMMAND_UUID = '65ecdf0d-cde0-4543-a62b-c166c3341319';
export const FOCUS_TIMER_RESPONSE_UUID = '2c4e304b-2581-481a-8646-89122d760711';

const FALLBACK_GATT_VALUE_BYTES = 20;
const MAXIMUM_GATT_VALUE_BYTES = 182;
const STATE_SETTLE_MS = 2_000;
const ACCEPTANCE_SUBSCRIPTION_SETTLE_MS = 100;

type RejectInFlight = (error: DeviceTransportError) => void;

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : 'Unknown Bluetooth error';
}

function transportError(
  code: DeviceTransportErrorCode,
  retryable: boolean,
  context: string,
  cause?: unknown,
): DeviceTransportError {
  const suffix = cause === undefined ? '' : `: ${messageOf(cause)}`;
  return new DeviceTransportError(code, retryable, `${context}${suffix}`);
}

function availabilityFor(state: State): DeviceTransportAvailability {
  switch (state) {
    case State.PoweredOn:
      return { status: 'available' };
    case State.Unsupported:
      return { status: 'unavailable', reason: 'unsupported' };
    case State.Unauthorized:
      return { status: 'permission-denied', canOpenSettings: true };
    case State.PoweredOff:
    case State.Resetting:
    case State.Unknown:
      return { status: 'unavailable', reason: 'powered-off' };
  }
}

function gattValueCapacity(device: Device): number {
  const negotiated = Number.isFinite(device.mtu) ? device.mtu - 3 : FALLBACK_GATT_VALUE_BYTES;
  return Math.max(BLE_FRAME_HEADER_BYTES + 1, Math.min(MAXIMUM_GATT_VALUE_BYTES, negotiated));
}

export class BleDeviceTransport implements DeviceTransport {
  readonly kind = 'ble';

  private connected: Device | null = null;
  private disconnectSubscription: Subscription | null = null;
  private explicitDisconnect = false;
  private nextRequestTransferId = 0;
  private requestInFlight = false;
  private rejectInFlight: RejectInFlight | null = null;
  private transactionSequence = 0;
  private readonly disconnectListeners = new Set<(event: DeviceTransportDisconnect) => void>();

  constructor(private readonly manager = new BleManager()) {}

  async readAvailability(): Promise<DeviceTransportAvailability> {
    const current = await this.manager.state();
    if (current !== State.Unknown && current !== State.Resetting) {
      return availabilityFor(current);
    }

    return await new Promise((resolve) => {
      let settled = false;
      let subscription: Subscription | undefined;
      let timer: ReturnType<typeof setTimeout> | undefined;
      const finish = (state: State) => {
        if (settled) return;
        settled = true;
        if (timer !== undefined) clearTimeout(timer);
        subscription?.remove();
        resolve(availabilityFor(state));
      };
      subscription = this.manager.onStateChange((state) => {
        if (state !== State.Unknown && state !== State.Resetting) finish(state);
      }, true);
      if (settled) {
        subscription.remove();
        return;
      }
      timer = setTimeout(() => finish(current), STATE_SETTLE_MS);
    });
  }

  async scan(operation: DeviceTransportOperation): Promise<readonly DeviceCandidate[]> {
    this.assertOperation(operation, 'scan-failed');
    const found = new Map<string, DeviceCandidate>();

    return await new Promise((resolve, reject) => {
      let settled = false;
      let cancellationCleanup: (() => void) | undefined;
      const finish = (error?: DeviceTransportError) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        cancellationCleanup?.();
        void this.manager.stopDeviceScan().catch(() => undefined);
        if (error === undefined) resolve([...found.values()]);
        else reject(error);
      };
      const timer = setTimeout(() => finish(), operation.timeoutMs);
      cancellationCleanup = operation.cancellation?.subscribe(() => {
        finish(transportError('cancelled', false, 'Bluetooth scan was cancelled'));
      });

      void this.manager
        .startDeviceScan([FOCUS_TIMER_SERVICE_UUID], null, (error, device) => {
          if (error !== null) {
            finish(transportError('scan-failed', true, 'Bluetooth scan failed', error));
            return;
          }
          if (device === null) return;
          found.set(device.id, {
            transportId: device.id,
            productName: device.localName ?? device.name,
            rssi: device.rssi,
          });
          // A Focus Timer is the only MVP candidate; stop radio scanning promptly.
          finish();
        })
        .catch((error: unknown) => {
          finish(transportError('scan-failed', true, 'Bluetooth scan could not start', error));
        });
    });
  }

  async connect(candidate: DeviceCandidate, operation: DeviceTransportOperation): Promise<void> {
    this.assertOperation(operation, 'connect-failed');
    if (this.connected?.id === candidate.transportId) {
      if (await this.connected.isConnected()) return;
      this.connected = null;
    }
    if (this.connected !== null) await this.disconnect();

    let cancellationCleanup: (() => void) | undefined;
    const cancelled = new Promise<never>((_, reject) => {
      cancellationCleanup = operation.cancellation?.subscribe(() => {
        void this.manager.cancelDeviceConnection(candidate.transportId).catch(() => undefined);
        reject(transportError('cancelled', false, 'Bluetooth connection was cancelled'));
      });
    });

    try {
      const connected = await this.withTimeout(
        Promise.race([
          this.manager.connectToDevice(candidate.transportId, { timeout: operation.timeoutMs }),
          cancelled,
        ]),
        operation.timeoutMs,
        'connect-failed',
        'Bluetooth connection timed out',
      );
      const discovered = await this.withTimeout(
        connected.discoverAllServicesAndCharacteristics(),
        operation.timeoutMs,
        'connect-failed',
        'Bluetooth service discovery timed out',
      );
      const characteristics = await discovered.characteristicsForService(FOCUS_TIMER_SERVICE_UUID);
      const command = characteristics.find(
        ({ uuid }) => uuid.toLowerCase() === FOCUS_TIMER_COMMAND_UUID,
      );
      const response = characteristics.find(
        ({ uuid }) => uuid.toLowerCase() === FOCUS_TIMER_RESPONSE_UUID,
      );
      if (command?.isWritableWithResponse !== true) {
        throw transportError(
          'connect-failed',
          false,
          'Focus Timer command characteristic is missing or not writable',
        );
      }
      if (response === undefined || (!response.isNotifiable && !response.isIndicatable)) {
        throw transportError(
          'connect-failed',
          false,
          'Focus Timer response characteristic is missing or not notifiable',
        );
      }

      this.connected = discovered;
      this.installDisconnectMonitor(discovered);
    } catch (error) {
      void this.manager.cancelDeviceConnection(candidate.transportId).catch(() => undefined);
      if (error instanceof DeviceTransportError) throw error;
      throw transportError('connect-failed', true, 'Unable to connect to Focus Timer', error);
    } finally {
      cancellationCleanup?.();
    }
  }

  async disconnect(): Promise<void> {
    const connected = this.connected;
    this.connected = null;
    this.rejectActiveRequest(
      transportError('connection-lost', true, 'Bluetooth connection was closed'),
    );
    this.disconnectSubscription?.remove();
    this.disconnectSubscription = null;
    if (connected === null) return;

    this.explicitDisconnect = true;
    try {
      await this.manager.cancelDeviceConnection(connected.id);
    } catch (error) {
      throw transportError('transport-failed', true, 'Unable to disconnect Focus Timer', error);
    } finally {
      this.explicitDisconnect = false;
    }
  }

  async request(payload: Uint8Array, operation: DeviceTransportOperation): Promise<Uint8Array> {
    this.assertOperation(operation, 'request-timeout');
    const connected = this.connected;
    if (connected === null || !(await connected.isConnected())) {
      throw transportError('not-connected', true, 'Connect before sending a request');
    }
    if (this.requestInFlight) {
      throw transportError('transport-failed', true, 'Only one Bluetooth request may be in flight');
    }

    this.requestInFlight = true;
    this.nextRequestTransferId = nextTransferId(this.nextRequestTransferId);
    const transferId = this.nextRequestTransferId;
    const requestId = `focus-${++this.transactionSequence}-${transferId}`;
    const monitorId = `${requestId}-notify`;
    const writeIds: string[] = [];
    const reassembler = new BleReassembler();
    let monitor: Subscription | undefined;
    let cancellationCleanup: (() => void) | undefined;
    let timeout: ReturnType<typeof setTimeout> | undefined;

    try {
      const response = new Promise<Uint8Array>((resolve, reject) => {
        let settled = false;
        const finish = (error?: DeviceTransportError, value?: Uint8Array) => {
          if (settled) return;
          settled = true;
          this.rejectInFlight = null;
          if (error !== undefined) reject(error);
          else resolve(value!);
        };
        this.rejectInFlight = (error) => finish(error);
        timeout = setTimeout(
          () => finish(transportError('request-timeout', true, 'Bluetooth request timed out')),
          operation.timeoutMs,
        );
        cancellationCleanup = operation.cancellation?.subscribe(() => {
          finish(transportError('cancelled', false, 'Bluetooth request was cancelled'));
        });

        // Notifications are subscribed before the first command fragment is written.
        monitor = connected.monitorCharacteristicForService(
          FOCUS_TIMER_SERVICE_UUID,
          FOCUS_TIMER_RESPONSE_UUID,
          (error: BleError | null, characteristic) => {
            if (error !== null) {
              finish(
                transportError('transport-failed', true, 'Focus Timer notification failed', error),
              );
              return;
            }
            if (characteristic?.value === null || characteristic?.value === undefined) {
              finish(
                transportError('transport-failed', true, 'Focus Timer sent an empty notification'),
              );
              return;
            }
            try {
              const frame = base64ToBytes(characteristic.value);
              if (parseBleFrame(frame).header.transferId !== transferId) return;
              const result = reassembler.acceptFrame(frame, Date.now());
              if (result.status === 'complete') finish(undefined, result.message);
            } catch (cause) {
              finish(
                transportError(
                  'transport-failed',
                  true,
                  'Focus Timer sent a malformed response frame',
                  cause,
                ),
              );
            }
          },
          monitorId,
        );
      });

      const fragmenter = new BleFragmenter(payload, transferId, gattValueCapacity(connected));
      let frameIndex = 0;
      for (
        let frame = fragmenter.nextFrame();
        frame !== undefined;
        frame = fragmenter.nextFrame()
      ) {
        this.assertOperation(operation, 'request-timeout');
        const writeId = `${requestId}-write-${frameIndex++}`;
        writeIds.push(writeId);
        await this.withTimeout(
          connected.writeCharacteristicWithResponseForService(
            FOCUS_TIMER_SERVICE_UUID,
            FOCUS_TIMER_COMMAND_UUID,
            bytesToBase64(frame),
            writeId,
          ),
          operation.timeoutMs,
          'request-timeout',
          'Bluetooth command write timed out',
        );
      }
      return await response;
    } catch (error) {
      if (error instanceof DeviceTransportError) throw error;
      throw transportError('transport-failed', true, 'Bluetooth request failed', error);
    } finally {
      if (timeout !== undefined) clearTimeout(timeout);
      cancellationCleanup?.();
      monitor?.remove();
      reassembler.reset();
      this.rejectInFlight = null;
      this.requestInFlight = false;
      void this.manager.cancelTransaction(monitorId).catch(() => undefined);
      for (const writeId of writeIds) {
        void this.manager.cancelTransaction(writeId).catch(() => undefined);
      }
    }
  }

  /** Sends one deliberately invalid GATT frame for an explicit bench run. */
  async writeCorruptFrameForAcceptance(
    operation: DeviceTransportOperation,
  ): Promise<'gatt-rejected' | 'written'> {
    this.assertOperation(operation, 'request-timeout');
    const connected = this.connected;
    if (connected === null || !(await connected.isConnected())) {
      throw transportError('not-connected', true, 'Connect before running BLE acceptance');
    }
    if (this.requestInFlight) {
      throw transportError('transport-failed', true, 'A Bluetooth request is already active');
    }

    this.requestInFlight = true;
    const transactionId = `focus-acceptance-corrupt-${++this.transactionSequence}`;
    const monitor = connected.monitorCharacteristicForService(
      FOCUS_TIMER_SERVICE_UUID,
      FOCUS_TIMER_RESPONSE_UUID,
      () => undefined,
      `${transactionId}-notify`,
    );
    try {
      await delay(ACCEPTANCE_SUBSCRIPTION_SETTLE_MS);
      try {
        await connected.writeCharacteristicWithResponseForService(
          FOCUS_TIMER_SERVICE_UUID,
          FOCUS_TIMER_COMMAND_UUID,
          bytesToBase64(Uint8Array.of(0xff)),
          transactionId,
        );
        return 'written';
      } catch {
        return 'gatt-rejected';
      }
    } finally {
      monitor.remove();
      this.requestInFlight = false;
      void this.manager.cancelTransaction(transactionId).catch(() => undefined);
      await delay(ACCEPTANCE_SUBSCRIPTION_SETTLE_MS);
    }
  }

  /**
   * Disconnects after the first response notification of a multi-frame page.
   * This deliberately exercises the firmware's dropped-notification path.
   */
  async disconnectDuringResponseForAcceptance(
    payload: Uint8Array,
    operation: DeviceTransportOperation,
  ): Promise<void> {
    this.assertOperation(operation, 'request-timeout');
    const connected = this.connected;
    if (connected === null || !(await connected.isConnected())) {
      throw transportError('not-connected', true, 'Connect before running BLE acceptance');
    }
    if (this.requestInFlight) {
      throw transportError('transport-failed', true, 'A Bluetooth request is already active');
    }

    this.requestInFlight = true;
    this.nextRequestTransferId = nextTransferId(this.nextRequestTransferId);
    const transferId = this.nextRequestTransferId;
    const transactionId = `focus-acceptance-drop-${++this.transactionSequence}-${transferId}`;
    let monitor: Subscription | undefined;
    let settled = false;
    let resolveDrop: (() => void) | undefined;
    let rejectDrop: ((error: DeviceTransportError) => void) | undefined;
    const firstNotification = new Promise<void>((resolve, reject) => {
      resolveDrop = resolve;
      rejectDrop = reject;
    });

    try {
      monitor = connected.monitorCharacteristicForService(
        FOCUS_TIMER_SERVICE_UUID,
        FOCUS_TIMER_RESPONSE_UUID,
        (error, characteristic) => {
          if (settled) return;
          if (error !== null) {
            settled = true;
            rejectDrop?.(
              transportError(
                'transport-failed',
                true,
                'Acceptance notification failed before disconnect',
                error,
              ),
            );
            return;
          }
          if (characteristic?.value === null || characteristic?.value === undefined) return;
          try {
            const frame = base64ToBytes(characteristic.value);
            if (parseBleFrame(frame).header.transferId !== transferId) return;
          } catch (cause) {
            settled = true;
            rejectDrop?.(
              transportError(
                'transport-failed',
                true,
                'Acceptance received a malformed notification',
                cause,
              ),
            );
            return;
          }

          settled = true;
          void this.manager
            .cancelDeviceConnection(connected.id)
            .then(() => {
              this.connected = null;
              resolveDrop?.();
            })
            .catch((cause: unknown) => {
              rejectDrop?.(
                transportError(
                  'transport-failed',
                  true,
                  'Acceptance could not disconnect after the first notification',
                  cause,
                ),
              );
            });
        },
        `${transactionId}-notify`,
      );
      await delay(ACCEPTANCE_SUBSCRIPTION_SETTLE_MS);

      const fragmenter = new BleFragmenter(payload, transferId, gattValueCapacity(connected));
      let frameIndex = 0;
      for (
        let frame = fragmenter.nextFrame();
        frame !== undefined;
        frame = fragmenter.nextFrame()
      ) {
        await connected.writeCharacteristicWithResponseForService(
          FOCUS_TIMER_SERVICE_UUID,
          FOCUS_TIMER_COMMAND_UUID,
          bytesToBase64(frame),
          `${transactionId}-write-${frameIndex++}`,
        );
      }
      await this.withTimeout(
        firstNotification,
        operation.timeoutMs,
        'request-timeout',
        'Acceptance did not receive the first response notification',
      );
    } finally {
      monitor?.remove();
      this.requestInFlight = false;
    }
  }

  subscribeToDisconnect(listener: (event: DeviceTransportDisconnect) => void): () => void {
    this.disconnectListeners.add(listener);
    return () => this.disconnectListeners.delete(listener);
  }

  subscribeToAvailability(listener: (state: DeviceTransportAvailability) => void): () => void {
    const subscription = this.manager.onStateChange(
      (state) => listener(availabilityFor(state)),
      true,
    );
    return () => subscription.remove();
  }

  private installDisconnectMonitor(device: Device): void {
    this.disconnectSubscription?.remove();
    this.disconnectSubscription = this.manager.onDeviceDisconnected(device.id, (error) => {
      if (this.explicitDisconnect) return;
      this.connected = null;
      const requestError = transportError(
        'connection-lost',
        true,
        'Focus Timer Bluetooth link was lost',
        error ?? undefined,
      );
      this.rejectActiveRequest(requestError);
      const event: DeviceTransportDisconnect = {
        reason: error === null ? 'remote' : 'link-loss',
        message: error?.message ?? null,
      };
      for (const listener of this.disconnectListeners) listener(event);
    });
  }

  private rejectActiveRequest(error: DeviceTransportError): void {
    this.rejectInFlight?.(error);
    this.rejectInFlight = null;
  }

  private assertOperation(
    operation: DeviceTransportOperation,
    timeoutCode: DeviceTransportErrorCode,
  ): void {
    if (!Number.isFinite(operation.timeoutMs) || operation.timeoutMs <= 0) {
      throw transportError(timeoutCode, true, 'Operation timeout must be positive');
    }
    if (operation.cancellation?.cancelled === true) {
      throw transportError('cancelled', false, 'Bluetooth operation was cancelled');
    }
  }

  private async withTimeout<T>(
    promise: Promise<T>,
    timeoutMs: number,
    timeoutCode: DeviceTransportErrorCode,
    timeoutMessage: string,
  ): Promise<T> {
    let timeout: ReturnType<typeof setTimeout> | undefined;
    try {
      return await Promise.race([
        promise,
        new Promise<never>((_, reject) => {
          timeout = setTimeout(
            () => reject(transportError(timeoutCode, true, timeoutMessage)),
            timeoutMs,
          );
        }),
      ]);
    } finally {
      if (timeout !== undefined) clearTimeout(timeout);
    }
  }
}

export function createBleDeviceTransport(): BleDeviceTransport {
  return new BleDeviceTransport();
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
