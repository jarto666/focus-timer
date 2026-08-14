export const PROTOCOL_MAJOR = 1 as const;
export const PROTOCOL_MINOR = 0 as const;

export const MAX_LOGICAL_MESSAGE_BYTES = 2_048;
export const MAX_RECORDS_PER_PAGE = 8;
export const MAX_CAPABILITIES = 8;
export const MAX_PRODUCT_NAME_BYTES = 24;
export const MAX_FIRMWARE_VERSION_BYTES = 32;
export const MAX_PRESET_ID_BYTES = 32;
export const MAX_PRESET_NAME_BYTES = 32;
export const MAX_SAFE_PROTOCOL_INTEGER = Number.MAX_SAFE_INTEGER;

export interface ProtocolVersion {
  readonly major: number;
  readonly minor: number;
}

export const protocolVersion: ProtocolVersion = {
  major: PROTOCOL_MAJOR,
  minor: PROTOCOL_MINOR,
};

export enum Capability {
  ReadStatus = 1,
  ReadSessionPages = 2,
  SetClockAnchor = 3,
}

export enum ViewState {
  Idle = 0,
  Running = 1,
  Paused = 2,
  Completed = 3,
}

export enum JournalHealth {
  Healthy = 0,
  Degraded = 1,
  Unavailable = 2,
}

export enum SessionOutcome {
  Completed = 0,
  Cancelled = 1,
}

export enum ProtocolErrorCode {
  MalformedEnvelope = 1,
  UnsupportedProtocolVersion = 2,
  UnsupportedMessage = 3,
  MalformedPayload = 4,
  InvalidField = 5,
  LimitExceeded = 6,
  HandshakeRequired = 7,
  Busy = 8,
  JournalUnavailable = 9,
  InternalError = 10,
}

export interface PresetSnapshot {
  readonly id: string;
  readonly name: string;
  readonly plannedDurationMs: number;
}

export interface HelloResponse {
  readonly deviceId: Uint8Array;
  readonly productName: string;
  readonly firmwareVersion: string;
  readonly supportedVersion: ProtocolVersion;
  readonly capabilities: readonly Capability[];
}

export interface JournalStatus {
  readonly epoch: Uint8Array;
  readonly oldestSequence?: number;
  readonly latestSequence?: number;
  readonly health: JournalHealth;
}

export interface StatusResponse {
  readonly viewState: ViewState;
  readonly preset: PresetSnapshot;
  readonly remainingDurationMs: number;
  readonly journal: JournalStatus;
  readonly clockKnown: boolean;
}

export interface SessionPageRequest {
  readonly journalEpoch?: Uint8Array;
  readonly afterSequence: number;
  readonly limit: number;
}

export interface SessionRecord {
  readonly sequence: number;
  readonly preset: PresetSnapshot;
  readonly activeDurationMs: number;
  readonly outcome: SessionOutcome;
  readonly startedAtUtcMs?: number;
  readonly endedAtUtcMs?: number;
}

export interface SessionPageResponse {
  readonly journalEpoch: Uint8Array;
  readonly oldestSequence?: number;
  readonly latestSequence?: number;
  readonly gap: boolean;
  readonly epochChanged: boolean;
  readonly records: readonly SessionRecord[];
  readonly continuationAfterSequence?: number;
}

export interface ClockAnchorRequest {
  readonly utcMs: number;
}

export interface ClockAnchorResponse {
  readonly acceptedUtcMs: number;
  readonly deviceMonotonicMsAtReceipt: number;
}

export interface ErrorResponse {
  readonly code: ProtocolErrorCode;
  readonly failedMessageKind?: number;
  readonly fieldId?: number;
  readonly supportedVersion?: ProtocolVersion;
}

export type Request =
  | { readonly type: 'hello' }
  | { readonly type: 'getStatus' }
  | { readonly type: 'getSessionPage'; readonly page: SessionPageRequest }
  | { readonly type: 'setClockAnchor'; readonly anchor: ClockAnchorRequest }
  | { readonly type: 'unknown'; readonly messageKind: number };

export type Response =
  | { readonly type: 'hello'; readonly hello: HelloResponse }
  | { readonly type: 'status'; readonly status: StatusResponse }
  | { readonly type: 'sessionPage'; readonly page: SessionPageResponse }
  | { readonly type: 'clockAnchor'; readonly anchor: ClockAnchorResponse }
  | { readonly type: 'error'; readonly error: ErrorResponse };

export interface RequestEnvelope {
  readonly version: ProtocolVersion;
  readonly requestId: number;
  readonly request: Request;
}

export interface ResponseEnvelope {
  readonly version: ProtocolVersion;
  readonly requestId: number;
  readonly response: Response;
}
