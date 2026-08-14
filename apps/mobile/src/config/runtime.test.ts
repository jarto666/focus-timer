import { describe, expect, it } from 'vitest';

import { resolveDeviceBackend, resolveMockScenario } from './runtime';

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

describe('resolveMockScenario', () => {
  it.each([undefined, ''])('defaults %s to paged history', (value) => {
    expect(resolveMockScenario(value)).toBe('paged');
  });

  it('accepts a registered deterministic scenario', () => {
    expect(resolveMockScenario('changed-epoch')).toBe('changed-epoch');
  });

  it('rejects silent fallback for an unknown scenario', () => {
    expect(() => resolveMockScenario('random-demo')).toThrow(/expected one of/);
  });
});
