export const deviceClientPackageName = '@focus-timer/device-client';

export {
  connectionActionsFor,
  connectionPhases,
  isReadyConnection,
  type ConnectionActions,
  type ConnectionPhase,
  type DeviceConnectionState,
  type ProtocolVersion,
  type ReadyDevice,
  type RetryableConnectionError,
} from './connection-state';
export {
  DeviceTransportError,
  type DeviceCandidate,
  type DeviceTransport,
  type DeviceTransportAvailability,
  type DeviceTransportCancellation,
  type DeviceTransportDisconnect,
  type DeviceTransportErrorCode,
  type DeviceTransportOperation,
} from './device-transport';
