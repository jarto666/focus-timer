import { describe, expect, it } from 'vitest';

import { protocolVersion } from './index';

describe('protocolVersion', () => {
  it('starts at protocol major one', () => {
    expect(protocolVersion).toEqual({ major: 1, minor: 0 });
  });
});
