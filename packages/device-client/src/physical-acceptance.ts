import {
  MAX_RECORDS_PER_PAGE,
  PROTOCOL_MAJOR,
  PROTOCOL_MINOR,
  ProtocolErrorCode,
  decodeResponse,
  encodeRequest,
  type Request,
  type Response,
} from '@focus-timer/device-protocol';

import type { DeviceTransport, DeviceTransportOperation } from './device-transport';

const ACCEPTANCE_REQUEST_ID_BASE = 0xfa00_0000;

export type ProtocolFaultAcceptanceResult = Readonly<{
  duplicatePageWasStable: boolean;
  incompatibleVersionRejected: boolean;
  unknownCommandRejected: boolean;
  malformedPayloadRejected: boolean;
  validRequestRecovered: boolean;
}>;

/**
 * Development-build-only physical protocol probe.
 *
 * The caller owns connection and UI lifecycle. Every logical request is
 * read-only; the malformed payload is expected to time out or be rejected.
 * A final status request proves that bounded failure handling did not poison
 * the active protocol session.
 */
export async function runProtocolFaultAcceptance(
  transport: DeviceTransport,
  operation: DeviceTransportOperation,
): Promise<ProtocolFaultAcceptanceResult> {
  const hello = await exchange(
    transport,
    ACCEPTANCE_REQUEST_ID_BASE,
    { type: 'hello' },
    { major: PROTOCOL_MAJOR, minor: PROTOCOL_MINOR },
    operation,
  );
  assertResponseType(hello, 'hello');

  const duplicatePageRequest = encodeRequest({
    version: { major: PROTOCOL_MAJOR, minor: PROTOCOL_MINOR },
    requestId: ACCEPTANCE_REQUEST_ID_BASE + 1,
    request: {
      type: 'getSessionPage',
      page: { afterSequence: 0, limit: MAX_RECORDS_PER_PAGE },
    },
  });
  const firstDuplicatePage = await transport.request(duplicatePageRequest, operation);
  const secondDuplicatePage = await transport.request(duplicatePageRequest, operation);
  const duplicatePageWasStable = bytesEqual(firstDuplicatePage, secondDuplicatePage);
  assertResponseType(decodeResponse(firstDuplicatePage).response, 'sessionPage');
  assertResponseType(decodeResponse(secondDuplicatePage).response, 'sessionPage');

  const incompatible = await exchange(
    transport,
    ACCEPTANCE_REQUEST_ID_BASE + 2,
    { type: 'hello' },
    { major: PROTOCOL_MAJOR + 1, minor: 0 },
    operation,
  );
  const incompatibleVersionRejected =
    incompatible.type === 'error' &&
    incompatible.error.code === ProtocolErrorCode.UnsupportedProtocolVersion;

  const unknown = await exchange(
    transport,
    ACCEPTANCE_REQUEST_ID_BASE + 3,
    { type: 'unknown', messageKind: 0xfe },
    { major: PROTOCOL_MAJOR, minor: PROTOCOL_MINOR },
    operation,
  );
  const unknownCommandRejected =
    unknown.type === 'error' && unknown.error.code === ProtocolErrorCode.UnsupportedMessage;

  let malformedPayloadRejected = false;
  try {
    await transport.request(Uint8Array.of(0xff), {
      ...operation,
      timeoutMs: Math.min(operation.timeoutMs, 1_000),
    });
  } catch {
    malformedPayloadRejected = true;
  }

  const recovered = await exchange(
    transport,
    ACCEPTANCE_REQUEST_ID_BASE + 4,
    { type: 'getStatus' },
    { major: PROTOCOL_MAJOR, minor: PROTOCOL_MINOR },
    operation,
  );
  const validRequestRecovered = recovered.type === 'status';

  const result = {
    duplicatePageWasStable,
    incompatibleVersionRejected,
    unknownCommandRejected,
    malformedPayloadRejected,
    validRequestRecovered,
  };
  if (Object.values(result).some((passed) => !passed)) {
    throw new Error(`Physical protocol fault acceptance failed: ${JSON.stringify(result)}`);
  }
  return result;
}

async function exchange(
  transport: DeviceTransport,
  requestId: number,
  request: Request,
  version: Readonly<{ major: number; minor: number }>,
  operation: DeviceTransportOperation,
): Promise<Response> {
  const response = decodeResponse(
    await transport.request(encodeRequest({ version, requestId, request }), operation),
  );
  if (response.requestId !== requestId) {
    throw new Error(`Acceptance response correlation mismatch for ${requestId}`);
  }
  return response.response;
}

function assertResponseType<T extends Response['type']>(
  response: Response,
  expected: T,
): asserts response is Extract<Response, { type: T }> {
  if (response.type !== expected) {
    throw new Error(`Expected ${expected}, received ${response.type}`);
  }
}

function bytesEqual(left: Uint8Array, right: Uint8Array): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}
