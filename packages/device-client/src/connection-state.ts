import type { DeviceCandidate } from './device-transport';

export const connectionPhases = [
  'unavailable',
  'permission-denied',
  'scanning',
  'connecting',
  'handshaking',
  'ready',
  'disconnected',
  'incompatible',
  'retryable-error',
] as const;

export type ConnectionPhase = (typeof connectionPhases)[number];

export type ProtocolVersion = Readonly<{
  major: number;
  minor: number;
}>;

export type ReadyDevice = Readonly<{
  /** Stable product identity returned by the protocol handshake. */
  deviceId: string;
  /** Transport-specific reconnection hint; never a product identity. */
  transportId: string;
  productName: string | null;
  firmwareVersion: string;
  protocolVersion: ProtocolVersion;
}>;

export type RetryableConnectionError = Readonly<{
  code:
    | 'scan-failed'
    | 'connect-failed'
    | 'handshake-timeout'
    | 'connection-lost'
    | 'request-timeout'
    | 'transport-failed';
  operation: 'scan' | 'connect' | 'handshake' | 'request' | 'disconnect';
  message: string;
}>;

export type DeviceConnectionState =
  | Readonly<{
      phase: 'unavailable';
      reason: 'powered-off' | 'unsupported';
    }>
  | Readonly<{
      phase: 'permission-denied';
      canOpenSettings: boolean;
    }>
  | Readonly<{
      phase: 'scanning';
      candidates: readonly DeviceCandidate[];
    }>
  | Readonly<{
      phase: 'connecting';
      candidate: DeviceCandidate;
    }>
  | Readonly<{
      phase: 'handshaking';
      candidate: DeviceCandidate;
    }>
  | Readonly<{
      phase: 'ready';
      device: ReadyDevice;
    }>
  | Readonly<{
      phase: 'disconnected';
      reason: 'initial' | 'user' | 'link-loss';
      lastDevice: ReadyDevice | null;
    }>
  | Readonly<{
      phase: 'incompatible';
      candidate: DeviceCandidate;
      supportedMajor: number;
      receivedMajor: number;
    }>
  | Readonly<{
      phase: 'retryable-error';
      candidate: DeviceCandidate | null;
      error: RetryableConnectionError;
    }>;

export type ConnectionActions = Readonly<{
  canScan: boolean;
  canRetry: boolean;
  canCancel: boolean;
  canDisconnect: boolean;
  canRequest: boolean;
}>;

const noActions: ConnectionActions = {
  canScan: false,
  canRetry: false,
  canCancel: false,
  canDisconnect: false,
  canRequest: false,
};

export function connectionActionsFor(state: DeviceConnectionState): ConnectionActions {
  switch (state.phase) {
    case 'unavailable':
      return { ...noActions, canRetry: state.reason === 'powered-off' };
    case 'permission-denied':
      return { ...noActions, canRetry: true };
    case 'disconnected':
    case 'incompatible':
      return { ...noActions, canScan: true };
    case 'scanning':
    case 'connecting':
    case 'handshaking':
      return { ...noActions, canCancel: true };
    case 'ready':
      return { ...noActions, canDisconnect: true, canRequest: true };
    case 'retryable-error':
      return { ...noActions, canRetry: true };
  }
}

export function isReadyConnection(
  state: DeviceConnectionState,
): state is Extract<DeviceConnectionState, { phase: 'ready' }> {
  return state.phase === 'ready';
}
