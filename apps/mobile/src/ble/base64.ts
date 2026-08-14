const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

/** Encode BLE bytes without depending on Node's Buffer in React Native. */
export function bytesToBase64(bytes: Uint8Array): string {
  let encoded = '';
  for (let offset = 0; offset < bytes.length; offset += 3) {
    const first = bytes[offset]!;
    const second = bytes[offset + 1];
    const third = bytes[offset + 2];
    const value = first * 0x1_0000 + (second ?? 0) * 0x100 + (third ?? 0);

    encoded += alphabet[Math.floor(value / 0x4_0000)]!;
    encoded += alphabet[Math.floor(value / 0x1_000) & 0x3f]!;
    encoded += second === undefined ? '=' : alphabet[Math.floor(value / 0x40) & 0x3f]!;
    encoded += third === undefined ? '=' : alphabet[value & 0x3f]!;
  }
  return encoded;
}

/** Decode the canonical Base64 values returned by react-native-ble-plx. */
export function base64ToBytes(encoded: string): Uint8Array {
  if (encoded.length === 0) return new Uint8Array();
  if (encoded.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(encoded)) {
    throw new TypeError('Invalid Base64 BLE value');
  }

  const padding = encoded.endsWith('==') ? 2 : encoded.endsWith('=') ? 1 : 0;
  if (encoded.slice(0, -padding || undefined).includes('=')) {
    throw new TypeError('Invalid Base64 padding');
  }
  const decoded = new Uint8Array((encoded.length / 4) * 3 - padding);
  let output = 0;

  for (let offset = 0; offset < encoded.length; offset += 4) {
    const values = [0, 1, 2, 3].map((index) => {
      const character = encoded[offset + index]!;
      if (character === '=') return 0;
      const value = alphabet.indexOf(character);
      if (value < 0) throw new TypeError('Invalid Base64 character');
      return value;
    });
    const packed = values[0]! * 0x4_0000 + values[1]! * 0x1_000 + values[2]! * 0x40 + values[3]!;
    if (output < decoded.length) decoded[output++] = Math.floor(packed / 0x1_0000) & 0xff;
    if (output < decoded.length) decoded[output++] = Math.floor(packed / 0x100) & 0xff;
    if (output < decoded.length) decoded[output++] = packed & 0xff;
  }
  return decoded;
}
