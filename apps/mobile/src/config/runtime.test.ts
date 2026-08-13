import { describe, expect, it } from 'vitest';

import { resolveDeviceBackend } from './runtime';

describe('resolveDeviceBackend', () => {
  it.each([undefined, '', 'mock'])('defaults %s to the deterministic mock', (value) => {
    expect(resolveDeviceBackend(value)).toBe('mock');
  });

  it('accepts the physical BLE composition', () => {
    expect(resolveDeviceBackend('ble')).toBe('ble');
  });

  it('rejects silent fallback for misspelled backends', () => {
    expect(() => resolveDeviceBackend('bluetooth')).toThrow(/expected "mock" or "ble"/);
  });
});
