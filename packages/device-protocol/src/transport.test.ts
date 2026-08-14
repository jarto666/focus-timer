import { describe, expect, it } from 'vitest';

import {
  BLE_FRAME_HEADER_BYTES,
  BLE_REASSEMBLY_TIMEOUT_MS,
  BleFragmentError,
  BleFragmenter,
  BleReassembler,
  crc32IsoHdlc,
  nextTransferId,
} from './index';

const bytes = (value: string): Uint8Array =>
  Uint8Array.from([...value].map((character) => character.charCodeAt(0)));

const frames = (
  message: Uint8Array,
  transferId: number,
  maximumFrameBytes: number,
): Uint8Array[] => {
  const fragmenter = new BleFragmenter(message, transferId, maximumFrameBytes);
  const result: Uint8Array[] = [];
  for (let frame = fragmenter.nextFrame(); frame !== undefined; frame = fragmenter.nextFrame()) {
    result.push(frame);
  }
  return result;
};

describe('BLE fragmentation transport', () => {
  it('matches the CRC-32/ISO-HDLC check value', () => {
    expect(crc32IsoHdlc(bytes('123456789'))).toBe(0xcbf4_3926);
  });

  it('round-trips single and multi-frame logical messages', () => {
    for (const [message, maximumFrameBytes] of [
      [bytes('hello'), 64],
      [bytes('a longer logical message'), BLE_FRAME_HEADER_BYTES + 5],
    ] as const) {
      const source = frames(message, 7, maximumFrameBytes);
      const reassembler = new BleReassembler();
      let result;
      for (const [index, frame] of source.entries())
        result = reassembler.acceptFrame(frame, index * 10);
      expect(result).toEqual({ status: 'complete', message });
    }
  });

  it('reassembles response and event streams independently', () => {
    const response = frames(bytes('correlated response'), 21, BLE_FRAME_HEADER_BYTES + 4);
    const event = frames(bytes('unsolicited event'), 22, BLE_FRAME_HEADER_BYTES + 3);
    const responseChannel = new BleReassembler();
    const eventChannel = new BleReassembler();
    let responseResult;
    let eventResult;
    for (let index = 0; index < Math.max(response.length, event.length); index += 1) {
      if (response[index] !== undefined)
        responseResult = responseChannel.acceptFrame(response[index]!, index);
      if (event[index] !== undefined) eventResult = eventChannel.acceptFrame(event[index]!, index);
    }
    expect(responseResult).toEqual({ status: 'complete', message: bytes('correlated response') });
    expect(eventResult).toEqual({ status: 'complete', message: bytes('unsolicited event') });
  });

  it('recovers after a duplicate start frame', () => {
    const source = frames(bytes('duplicate start transfer'), 31, BLE_FRAME_HEADER_BYTES + 4);
    const reassembler = new BleReassembler();
    expect(reassembler.acceptFrame(source[0]!, 0)).toEqual({ status: 'inProgress' });
    expect(() => reassembler.acceptFrame(source[0]!, 1)).toThrowError(
      expect.objectContaining({ code: 'duplicateStart' }),
    );
    const [retry] = frames(bytes('recovered'), 32, 64);
    expect(reassembler.acceptFrame(retry!, 2)).toEqual({
      status: 'complete',
      message: bytes('recovered'),
    });
  });

  it('writes the correlated header in big-endian order', () => {
    const [frame] = frames(bytes('abc'), 0x1234, 64);
    expect(frame).toBeDefined();
    expect(Array.from(frame!.subarray(2, 4))).toEqual([0x12, 0x34]);
    expect(Array.from(frame!.subarray(4, 6))).toEqual([0, 3]);
    expect(Array.from(frame!.subarray(6, 8))).toEqual([0, 0]);
    expect(Array.from(frame!.subarray(8, 12))).toEqual([0x35, 0x24, 0x41, 0xc2]);
  });

  it('resets on timeout and non-contiguous offsets', () => {
    const source = frames(bytes('abcdefghij'), 9, BLE_FRAME_HEADER_BYTES + 4);
    const reassembler = new BleReassembler();
    expect(reassembler.acceptFrame(source[0]!, 10)).toEqual({ status: 'inProgress' });
    expect(() =>
      reassembler.acceptFrame(source[1]!, 10 + BLE_REASSEMBLY_TIMEOUT_MS + 1),
    ).toThrowError(expect.objectContaining({ code: 'timedOut' }));

    expect(reassembler.acceptFrame(source[0]!, 20_000)).toEqual({ status: 'inProgress' });
    expect(() => reassembler.acceptFrame(source[2]!, 20_010)).toThrowError(
      expect.objectContaining({ code: 'nonContiguousOffset' }),
    );
  });

  it('rejects checksum corruption without exposing bytes', () => {
    const [frame] = frames(bytes('checksum'), 11, 64);
    expect(frame).toBeDefined();
    const lastIndex = frame!.length - 1;
    frame![lastIndex] = frame![lastIndex]! ^ 0x01;
    expect(() => new BleReassembler().acceptFrame(frame!, 0)).toThrowError(
      expect.objectContaining({ code: 'checksumMismatch' }),
    );
  });

  it('bounds inputs and increments transfer identifiers without zero', () => {
    expect(() => new BleFragmenter(new Uint8Array(), 1, 20)).toThrowError(BleFragmentError);
    expect(() => new BleFragmenter(bytes('x'), 0, 20)).toThrowError(
      expect.objectContaining({ code: 'invalidTransferId' }),
    );
    expect(() => new BleFragmenter(bytes('x'), 1, BLE_FRAME_HEADER_BYTES)).toThrowError(
      expect.objectContaining({ code: 'frameCapacityTooSmall' }),
    );
    expect(nextTransferId(1)).toBe(2);
    expect(nextTransferId(0xffff)).toBe(1);
  });
});
