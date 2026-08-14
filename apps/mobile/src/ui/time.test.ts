import { describe, expect, it } from 'vitest';

import { formatCountdown, formatElapsedDuration, resolveSessionMoment } from './time';

describe('mobile time presentation', () => {
  it('uses the same partial-second countdown rule as the physical OLED', () => {
    expect(formatCountdown(60_000)).toBe('01:00');
    expect(formatCountdown(59_999)).toBe('01:00');
    expect(formatCountdown(59_000)).toBe('00:59');
    expect(formatCountdown(1)).toBe('00:01');
    expect(formatCountdown(0)).toBe('00:00');
  });

  it('does not present a non-zero short session as zero minutes', () => {
    expect(formatElapsedDuration(1)).toBe('1s');
    expect(formatElapsedDuration(59_499)).toBe('59s');
    expect(formatElapsedDuration(59_500)).toBe('1m');
    expect(formatElapsedDuration(60_000)).toBe('1m');
    expect(formatElapsedDuration(90_000)).toBe('1m 30s');
    expect(formatElapsedDuration(3_720_000)).toBe('1h 2m');
  });

  it('uses a known completion time when the clock became known after session start', () => {
    expect(resolveSessionMoment(100, 200)).toEqual({ kind: 'started', timestampMs: 100 });
    expect(resolveSessionMoment(null, 200)).toEqual({ kind: 'finished', timestampMs: 200 });
    expect(resolveSessionMoment(null, null)).toEqual({ kind: 'unknown', timestampMs: null });
  });
});
