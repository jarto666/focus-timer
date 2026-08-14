export type DeviceCandidate = Readonly<{
  /** Adapter-specific discovery/reconnection hint, such as an iOS peripheral id. */
  transportId: string;
  productName: string | null;
  rssi: number | null;
}>;

export type DeviceTransportAvailability =
  | Readonly<{ status: 'available' }>
  | Readonly<{
      status: 'unavailable';
      reason: 'powered-off' | 'unsupported';
    }>
  | Readonly<{
      status: 'permission-denied';
      canOpenSettings: boolean;
    }>;

export interface DeviceTransportCancellation {
  readonly cancelled: boolean;
  subscribe(listener: () => void): () => void;
}

export type DeviceTransportOperation = Readonly<{
  timeoutMs: number;
  cancellation?: DeviceTransportCancellation;
}>;

export type DeviceTransportDisconnect = Readonly<{
  reason: 'link-loss' | 'remote' | 'transport-error';
  message: string | null;
}>;

export type DeviceTransportErrorCode =
  | 'unavailable'
  | 'permission-denied'
  | 'scan-failed'
  | 'connect-failed'
  | 'not-connected'
  | 'connection-lost'
  | 'request-timeout'
  | 'cancelled'
  | 'transport-failed';

export class DeviceTransportError extends Error {
  override readonly name = 'DeviceTransportError';

  constructor(
    readonly code: DeviceTransportErrorCode,
    readonly retryable: boolean,
    message: string,
  ) {
    super(message);
  }
}

/**
 * Platform-neutral byte transport. It knows how to discover and reach a device,
 * but it does not decode protocol messages or decide handshake compatibility.
 */
export interface DeviceTransport {
  readonly kind: string;

  readAvailability(): Promise<DeviceTransportAvailability>;

  scan(operation: DeviceTransportOperation): Promise<readonly DeviceCandidate[]>;

  connect(candidate: DeviceCandidate, operation: DeviceTransportOperation): Promise<void>;

  disconnect(): Promise<void>;

  request(payload: Uint8Array, operation: DeviceTransportOperation): Promise<Uint8Array>;

  subscribeToAvailability(listener: (state: DeviceTransportAvailability) => void): () => void;

  subscribeToDisconnect(listener: (event: DeviceTransportDisconnect) => void): () => void;
}
