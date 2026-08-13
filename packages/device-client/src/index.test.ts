import { describe, expect, it } from 'vitest';

import { deviceClientPackageName } from './index';

describe('device client package', () => {
  it('exports its stable workspace name', () => {
    expect(deviceClientPackageName).toBe('@focus-timer/device-client');
  });
});
