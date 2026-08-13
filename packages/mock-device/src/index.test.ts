import { describe, expect, it } from 'vitest';

import { mockDevicePackageName } from './index';

describe('mock device package', () => {
  it('exports its stable workspace name', () => {
    expect(mockDevicePackageName).toBe('@focus-timer/mock-device');
  });
});
