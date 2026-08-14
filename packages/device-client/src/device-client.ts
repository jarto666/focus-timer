import {
  Capability,
  ProtocolErrorCode,
  PROTOCOL_MAJOR,
  PROTOCOL_MINOR,
  decodeResponse,
  encodeRequest,
  type ClockAnchorResponse,
  type HelloResponse,
  type Request,
  type Response,
  type SessionPageRequest,
  type SessionPageResponse,
  type StatusResponse,
} from '@focus-timer/device-protocol';

import type { DeviceTransport, DeviceTransportOperation } from './device-transport';

export type DeviceClientErrorCode =
  | 'busy'
  | 'not-handshaken'
  | 'incompatible'
  | 'correlation-mismatch'
  | 'unexpected-response'
  | 'remote-error';

export class DeviceClientError extends Error {
  override readonly name = 'DeviceClientError';

  constructor(
    readonly code: DeviceClientErrorCode,
    readonly details: Readonly<{
      protocolErrorCode?: ProtocolErrorCode;
      supportedMajor?: number;
      receivedMajor?: number;
    }>,
    message: string,
  ) {
    super(message);
  }
}

/**
 * Correlated logical protocol client. BLE framing belongs to the selected
 * `DeviceTransport`; this class has no native or React Native dependency.
 */
export class DeviceClient {
  private nextRequestId = 1;
  private requestInFlight = false;
  private negotiatedMinor: number | undefined;
  private helloResponse: HelloResponse | undefined;

  constructor(private readonly transport: DeviceTransport) {}

  get hello(): HelloResponse | undefined {
    return this.helloResponse;
  }

  resetSession(): void {
    this.negotiatedMinor = undefined;
    this.helloResponse = undefined;
    this.requestInFlight = false;
  }

  async handshake(operation: DeviceTransportOperation): Promise<HelloResponse> {
    const response = await this.exchange(
      { type: 'hello' },
      { major: PROTOCOL_MAJOR, minor: PROTOCOL_MINOR },
      operation,
    );
    if (response.type === 'error') {
      if (response.error.code === ProtocolErrorCode.UnsupportedProtocolVersion) {
        throw new DeviceClientError(
          'incompatible',
          {
            ...(response.error.supportedVersion === undefined
              ? {}
              : { supportedMajor: response.error.supportedVersion.major }),
            receivedMajor: PROTOCOL_MAJOR,
            protocolErrorCode: response.error.code,
          },
          'The timer uses an incompatible protocol version',
        );
      }
      throw remoteError(response);
    }
    if (response.type !== 'hello') {
      throw unexpected('hello', response.type);
    }
    if (response.hello.supportedVersion.major !== PROTOCOL_MAJOR) {
      throw new DeviceClientError(
        'incompatible',
        {
          supportedMajor: response.hello.supportedVersion.major,
          receivedMajor: PROTOCOL_MAJOR,
        },
        'The timer reported an incompatible protocol version',
      );
    }

    this.negotiatedMinor = Math.min(PROTOCOL_MINOR, response.hello.supportedVersion.minor);
    this.helloResponse = response.hello;
    return response.hello;
  }

  async getStatus(operation: DeviceTransportOperation): Promise<StatusResponse> {
    this.requireCapability(Capability.ReadStatus);
    const response = await this.exchangeReady({ type: 'getStatus' }, operation);
    if (response.type === 'error') throw remoteError(response);
    if (response.type !== 'status') throw unexpected('status', response.type);
    return response.status;
  }

  async getSessionPage(
    page: SessionPageRequest,
    operation: DeviceTransportOperation,
  ): Promise<SessionPageResponse> {
    this.requireCapability(Capability.ReadSessionPages);
    const response = await this.exchangeReady({ type: 'getSessionPage', page }, operation);
    if (response.type === 'error') throw remoteError(response);
    if (response.type !== 'sessionPage') throw unexpected('sessionPage', response.type);
    return response.page;
  }

  async setClockAnchor(
    utcMs: number,
    operation: DeviceTransportOperation,
  ): Promise<ClockAnchorResponse> {
    this.requireCapability(Capability.SetClockAnchor);
    const response = await this.exchangeReady(
      { type: 'setClockAnchor', anchor: { utcMs } },
      operation,
    );
    if (response.type === 'error') throw remoteError(response);
    if (response.type !== 'clockAnchor') throw unexpected('clockAnchor', response.type);
    return response.anchor;
  }

  private requireCapability(capability: Capability): void {
    if (this.negotiatedMinor === undefined || this.helloResponse === undefined) {
      throw new DeviceClientError('not-handshaken', {}, 'Handshake is required');
    }
    if (!this.helloResponse.capabilities.includes(capability)) {
      throw new DeviceClientError(
        'unexpected-response',
        {},
        `Timer did not advertise capability ${capability}`,
      );
    }
  }

  private exchangeReady(request: Request, operation: DeviceTransportOperation): Promise<Response> {
    if (this.negotiatedMinor === undefined) {
      throw new DeviceClientError('not-handshaken', {}, 'Handshake is required');
    }
    return this.exchange(
      request,
      { major: PROTOCOL_MAJOR, minor: this.negotiatedMinor },
      operation,
    );
  }

  private async exchange(
    request: Request,
    version: Readonly<{ major: number; minor: number }>,
    operation: DeviceTransportOperation,
  ): Promise<Response> {
    if (this.requestInFlight) {
      throw new DeviceClientError('busy', {}, 'Only one device request may be in flight');
    }
    const requestId = this.claimRequestId();
    this.requestInFlight = true;
    try {
      const encoded = encodeRequest({ version, requestId, request });
      const bytes = await this.transport.request(encoded, operation);
      const envelope = decodeResponse(bytes);
      if (envelope.requestId !== requestId) {
        throw new DeviceClientError(
          'correlation-mismatch',
          {},
          `Expected response ${requestId}, received ${envelope.requestId}`,
        );
      }
      const preHandshakeVersionError =
        request.type === 'hello' &&
        envelope.response.type === 'error' &&
        envelope.response.error.code === ProtocolErrorCode.UnsupportedProtocolVersion;
      if (!preHandshakeVersionError && envelope.version.major !== version.major) {
        throw new DeviceClientError(
          'incompatible',
          {
            supportedMajor: envelope.version.major,
            receivedMajor: version.major,
          },
          `Expected protocol major ${version.major}, received ${envelope.version.major}`,
        );
      }
      return envelope.response;
    } finally {
      this.requestInFlight = false;
    }
  }

  private claimRequestId(): number {
    const claimed = this.nextRequestId;
    this.nextRequestId = claimed === 0xffff_ffff ? 1 : claimed + 1;
    return claimed;
  }
}

function remoteError(response: Extract<Response, { type: 'error' }>): DeviceClientError {
  return new DeviceClientError(
    response.error.code === ProtocolErrorCode.UnsupportedProtocolVersion
      ? 'incompatible'
      : 'remote-error',
    {
      protocolErrorCode: response.error.code,
      ...(response.error.supportedVersion === undefined
        ? {}
        : { supportedMajor: response.error.supportedVersion.major }),
      receivedMajor: PROTOCOL_MAJOR,
    },
    `Timer rejected request with protocol error ${response.error.code}`,
  );
}

function unexpected(expected: string, received: string): DeviceClientError {
  return new DeviceClientError(
    'unexpected-response',
    {},
    `Expected ${expected} response, received ${received}`,
  );
}
