export type SessionMoment = Readonly<{
  kind: 'started' | 'finished' | 'unknown';
  timestampMs: number | null;
}>;

/** Matches the OLED's remaining-time rule: a partial second is still visible. */
export function formatCountdown(milliseconds: number): string {
  const totalSeconds = Math.max(0, Math.ceil(milliseconds / 1_000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
}

/** Rounds recorded elapsed time to the nearest second without collapsing it to `0m`. */
export function formatElapsedDuration(milliseconds: number): string {
  const roundedSeconds = Math.round(Math.max(0, milliseconds) / 1_000);
  const totalSeconds = milliseconds > 0 ? Math.max(1, roundedSeconds) : 0;

  if (totalSeconds < 60) {
    return `${totalSeconds}s`;
  }

  const hours = Math.floor(totalSeconds / 3_600);
  const minutes = Math.floor((totalSeconds % 3_600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return minutes === 0 ? `${hours}h` : `${hours}h ${minutes}m`;
  }
  return seconds === 0 ? `${minutes}m` : `${minutes}m ${seconds}s`;
}

/** Prefer the start instant, but retain a known completion instant for mid-session clock sync. */
export function resolveSessionMoment(
  startedAtUtcMs: number | null,
  endedAtUtcMs: number | null,
): SessionMoment {
  if (startedAtUtcMs !== null) {
    return { kind: 'started', timestampMs: startedAtUtcMs };
  }
  if (endedAtUtcMs !== null) {
    return { kind: 'finished', timestampMs: endedAtUtcMs };
  }
  return { kind: 'unknown', timestampMs: null };
}
