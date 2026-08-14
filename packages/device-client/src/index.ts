export const deviceClientPackageName = '@focus-timer/device-client';

export { DeviceClient, DeviceClientError, type DeviceClientErrorCode } from './device-client';
export {
  ForegroundSyncError,
  synchronizeForeground,
  type ForegroundSyncResult,
} from './foreground-sync';
export {
  bytesToHex,
  hexToBytes,
  nextCursorAfterSequence,
  type HistoryCompleteness,
  type JournalCursor,
  type KnownDeviceRecord,
  type PageCommit,
  type SessionRepository,
  type StoredSessionRecord,
} from './session-repository';

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
