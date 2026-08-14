import { describe, expect, it } from 'vitest';

import { base64ToBytes, bytesToBase64 } from './base64';

describe('React Native Base64 codec', () => {
  it.each([
    [[], ''],
    [[0], 'AA=='],
    [[0, 1], 'AAE='],
    [[0, 1, 2], 'AAEC'],
    [[0xff, 0xee, 0xdd, 0xcc], '/+7dzA=='],
  ] as const)('round-trips %j', (input, expected) => {
    const bytes = Uint8Array.from(input);
    expect(bytesToBase64(bytes)).toBe(expected);
    expect(base64ToBytes(expected)).toEqual(bytes);
  });

  it('round-trips every byte value across fragment boundaries', () => {
    const bytes = Uint8Array.from({ length: 256 }, (_, index) => index);
    expect(base64ToBytes(bytesToBase64(bytes))).toEqual(bytes);
  });

  it.each(['A', 'AA=A', '****', 'AAAA==='])('rejects malformed input %s', (encoded) => {
    expect(() => base64ToBytes(encoded)).toThrow(TypeError);
  });
});
