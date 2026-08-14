import { describe, expect, it } from 'vitest';

import {
  BLE_FRAME_HEADER_BYTES,
  BleFragmenter,
  BleReassembler,
  MAX_LOGICAL_MESSAGE_BYTES,
  ProtocolErrorCode,
  decodeRequest,
  decodeResponse,
  encodeResponse,
  protocolVersion,
  type ResponseEnvelope,
} from './index';

const hello = Uint8Array.from([0xa5, 0, 1, 1, 0, 2, 1, 3, 1, 4, 0xa0]);

const ascii = (value: string): Uint8Array =>
  Uint8Array.from([...value].map((character) => character.charCodeAt(0)));

const frames = (message: Uint8Array, transferId: number): Uint8Array[] => {
  const fragmenter = new BleFragmenter(message, transferId, BLE_FRAME_HEADER_BYTES + 4);
  const result: Uint8Array[] = [];
  for (let frame = fragmenter.nextFrame(); frame !== undefined; frame = fragmenter.nextFrame()) {
    result.push(frame);
  }
  return result;
};

const reassembleAll = (
  reassembler: BleReassembler,
  source: readonly Uint8Array[],
  startMs: number,
): void => {
  let result;
  for (const [index, frame] of source.entries())
    result = reassembler.acceptFrame(frame, startMs + index);
  expect(result).toEqual({ status: 'complete', message: expect.any(Uint8Array) });
};

describe('bounded malformed-input recovery', () => {
  it('rejects every truncated prefix and oversized logical input', () => {
    for (let length = 0; length < hello.length; length += 1) {
      expect(() => decodeRequest(hello.subarray(0, length))).toThrow();
    }
    expect(decodeRequest(hello)).toEqual({
      version: { major: 1, minor: 0 },
      requestId: 1,
      request: { type: 'hello' },
    });
    expect(() => decodeRequest(new Uint8Array(MAX_LOGICAL_MESSAGE_BYTES + 1))).toThrowError(
      expect.objectContaining({ code: 'messageTooLarge' }),
    );
  });

  it('reports duplicate keys and unsupported response kinds explicitly', () => {
    expect(() =>
      decodeRequest(Uint8Array.from([0xa5, 0, 1, 0, 0, 2, 1, 3, 1, 4, 0xa0])),
    ).toThrowError(expect.objectContaining({ code: 'duplicateOrUnorderedKey' }));
    expect(() =>
      decodeResponse(Uint8Array.from([0xa5, 0, 1, 1, 0, 2, 1, 3, 9, 4, 0xa0])),
    ).toThrowError(expect.objectContaining({ code: 'unsupportedMessage', field: 9 }));
  });

  it('preserves unknown requests and incompatible versions for bounded handler errors', () => {
    expect(decodeRequest(Uint8Array.from([0xa5, 0, 1, 1, 0, 2, 1, 3, 9, 4, 0xa0]))).toEqual({
      version: { major: 1, minor: 0 },
      requestId: 1,
      request: { type: 'unknown', messageKind: 9 },
    });
    const incompatible = decodeRequest(Uint8Array.from([0xa5, 0, 2, 1, 0, 2, 7, 3, 1, 4, 0xa0]));
    expect(incompatible.version.major).toBe(2);
    const response: ResponseEnvelope = {
      version: protocolVersion,
      requestId: incompatible.requestId,
      response: {
        type: 'error',
        error: {
          code: ProtocolErrorCode.UnsupportedProtocolVersion,
          failedMessageKind: 1,
          supportedVersion: protocolVersion,
        },
      },
    };
    expect(decodeResponse(encodeResponse(response))).toEqual(response);
  });

  it('recovers after out-of-order and checksum failures', () => {
    const message = ascii('a fragmented logical payload');
    const source = frames(message, 21);
    const reassembler = new BleReassembler();
    expect(reassembler.acceptFrame(source[0]!, 0)).toEqual({ status: 'inProgress' });
    expect(() => reassembler.acceptFrame(source[2]!, 1)).toThrowError(
      expect.objectContaining({ code: 'nonContiguousOffset' }),
    );
    reassembleAll(reassembler, source, 10);

    const valid = new BleFragmenter(ascii('checksum'), 22, 64).nextFrame();
    expect(valid).toBeDefined();
    const corrupted = valid!.slice();
    const lastIndex = corrupted.length - 1;
    corrupted[lastIndex] = corrupted[lastIndex]! ^ 1;
    expect(() => reassembler.acceptFrame(corrupted, 100)).toThrowError(
      expect.objectContaining({ code: 'checksumMismatch' }),
    );
    expect(reassembler.acceptFrame(valid!, 101)).toEqual({
      status: 'complete',
      message: ascii('checksum'),
    });
  });

  it('clears active state after malformed and oversized frames', () => {
    const source = frames(ascii('abcdefghij'), 31);
    const reassembler = new BleReassembler();
    expect(reassembler.acceptFrame(source[0]!, 0)).toEqual({ status: 'inProgress' });
    const reserved = source[1]!.slice();
    reserved[1] = reserved[1]! | 0x80;
    expect(() => reassembler.acceptFrame(reserved, 1)).toThrowError(
      expect.objectContaining({ code: 'reservedFlags' }),
    );
    expect(() => reassembler.acceptFrame(source[1]!, 2)).toThrowError(
      expect.objectContaining({ code: 'unexpectedFragment' }),
    );

    const oversized = source[0]!.slice();
    oversized[4] = 0x08;
    oversized[5] = 0x01;
    expect(() => reassembler.acceptFrame(oversized, 3)).toThrowError(
      expect.objectContaining({ code: 'invalidTotalLength' }),
    );
  });
});
