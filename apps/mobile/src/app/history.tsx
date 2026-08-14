import { SymbolView } from 'expo-symbols';
import { StyleSheet, Text, View } from 'react-native';

import type { HistoryCompleteness, LocalHistoryEntry } from '@/application/companion-model';
import { useCompanionRuntime } from '@/application/runtime-provider';
import { Page } from '@/ui/page';
import { Sigil } from '@/ui/sigil';
import { color, radius, space } from '@/ui/theme';

function formatDuration(milliseconds: number): string {
  const totalMinutes = Math.floor(milliseconds / 60_000);
  if (totalMinutes < 60) {
    return `${totalMinutes}m`;
  }
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return minutes === 0 ? `${hours}h` : `${hours}h ${minutes}m`;
}

function formatTimestamp(timestamp: number | null): string {
  if (timestamp === null) {
    return 'Time unavailable · device clock was unknown';
  }

  return new Intl.DateTimeFormat(undefined, {
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    month: 'short',
  }).format(new Date(timestamp));
}

function completenessCopy(completeness: HistoryCompleteness): string | null {
  switch (completeness) {
    case 'complete':
      return null;
    case 'retention-gap':
      return 'Some older device records were already removed before this sync. The sessions shown below are real, but this is not the complete history.';
    case 'changed-epoch':
      return 'The timer journal was reset since the previous cursor. Muninn started a new local segment without merging uncertain records.';
  }
}

function SessionCard({ entry }: Readonly<{ entry: LocalHistoryEntry }>) {
  const completed = entry.outcome === 'completed';
  return (
    <View style={styles.sessionCard}>
      <View style={styles.sessionRail}>
        <View style={[styles.outcomeMark, !completed && styles.outcomeMarkCancelled]} />
        <Text style={[styles.outcomeText, !completed && styles.outcomeTextCancelled]}>
          {entry.outcome.toUpperCase()}
        </Text>
        <Text style={styles.sequence}>#{String(entry.sequence).padStart(3, '0')}</Text>
      </View>
      <View style={styles.sessionBody}>
        <View style={styles.sessionCopy}>
          <Text style={styles.sessionTitle}>{entry.presetName}</Text>
          <Text style={styles.sessionTime}>{formatTimestamp(entry.startedAtUtcMs)}</Text>
        </View>
        <Text style={styles.sessionDuration}>{formatDuration(entry.activeDurationMs)}</Text>
      </View>
      <View style={styles.durationTrack}>
        <View
          style={[
            styles.durationFill,
            {
              width: `${Math.min(100, Math.round((entry.activeDurationMs / entry.plannedDurationMs) * 100))}%`,
            },
            !completed && styles.durationFillCancelled,
          ]}
        />
      </View>
    </View>
  );
}

export default function HistoryScreen() {
  const { history, historySync } = useCompanionRuntime();
  const totalActiveMs = history.entries.reduce((total, entry) => total + entry.activeDurationMs, 0);
  const incompleteCopy = completenessCopy(history.completeness);

  return (
    <Page>
      <View style={styles.header}>
        <Text style={styles.eyebrow}>LOCAL SESSION LEDGER</Text>
        <Text accessibilityRole="header" style={styles.title}>
          What the timer remembers.
        </Text>
        <Text style={styles.body}>
          Completed and cancelled sessions synchronized from the physical device.
        </Text>
      </View>

      <View style={styles.summaryRow}>
        <View style={styles.summaryCell}>
          <Text style={styles.summaryValue}>{String(history.entries.length).padStart(2, '0')}</Text>
          <Text style={styles.summaryLabel}>SESSIONS</Text>
        </View>
        <View style={styles.verticalRule} />
        <View style={styles.summaryCell}>
          <Text style={styles.summaryValue}>{formatDuration(totalActiveMs)}</Text>
          <Text style={styles.summaryLabel}>RECORDED FOCUS</Text>
        </View>
      </View>

      {historySync.phase === 'loading' || historySync.phase === 'syncing' ? (
        <View style={styles.syncNotice}>
          <View style={styles.syncPulse} />
          <View style={styles.syncCopy}>
            <Text style={styles.syncTitle}>
              {historySync.phase === 'loading' ? 'OPENING LOCAL LEDGER' : 'SYNCING DEVICE PAGES'}
            </Text>
            <Text style={styles.syncBody}>
              Existing entries remain readable while the next durable cursor is prepared.
            </Text>
          </View>
        </View>
      ) : null}

      {historySync.phase === 'failed' ? (
        <View style={styles.failedNotice}>
          <Text style={styles.failedCode}>SYNC INTERRUPTED</Text>
          <Text style={styles.failedBody}>{historySync.message}</Text>
          <Text style={styles.failedFoot}>
            Saved pages are intact. Retry from the device screen.
          </Text>
        </View>
      ) : null}

      {incompleteCopy === null ? null : (
        <View style={styles.incompleteNotice}>
          <Text style={styles.incompleteCode}>INCOMPLETE LEDGER</Text>
          <Text style={styles.incompleteBody}>{incompleteCopy}</Text>
        </View>
      )}

      <Text style={styles.sectionLabel}>RECENT</Text>
      {history.entries.length === 0 ? (
        <View style={styles.emptyCard}>
          <View style={styles.emptyIcon}>
            <Sigil name="norns" size={38} />
          </View>
          <View style={styles.emptyCopy}>
            <Text style={styles.emptyTitle}>The ledger is quiet</Text>
            <Text style={styles.emptyBody}>
              Connect a timer with finished sessions to begin this local record.
            </Text>
          </View>
        </View>
      ) : (
        <View style={styles.sessionList}>
          {history.entries.map((entry) => (
            <SessionCard entry={entry} key={entry.key} />
          ))}
        </View>
      )}

      <View style={styles.privacyNote}>
        <SymbolView
          fallback={<Text style={styles.privacyFallback}>✓</Text>}
          name="iphone.gen3"
          size={16}
          tintColor={color.faintText}
        />
        <Text style={styles.privacyText}>
          Durable on-device SQLite · no account · no cloud recovery
        </Text>
      </View>
    </Page>
  );
}

const styles = StyleSheet.create({
  header: { paddingBottom: space.xl, paddingTop: space.lg },
  eyebrow: {
    color: color.accent,
    fontSize: 10,
    fontWeight: '800',
    letterSpacing: 1.8,
    marginBottom: 9,
  },
  title: {
    color: color.text,
    fontSize: 34,
    fontWeight: '700',
    letterSpacing: -1.2,
    lineHeight: 40,
  },
  body: { color: color.mutedText, fontSize: 15, lineHeight: 22, marginTop: space.sm },
  summaryRow: {
    alignItems: 'center',
    backgroundColor: color.backgroundLifted,
    borderColor: color.line,
    borderRadius: radius.panel,
    borderWidth: 1,
    flexDirection: 'row',
    minHeight: 126,
  },
  summaryCell: { alignItems: 'center', flex: 1 },
  summaryValue: {
    color: color.text,
    fontSize: 32,
    fontVariant: ['tabular-nums'],
    fontWeight: '300',
  },
  summaryLabel: {
    color: color.faintText,
    fontSize: 9,
    fontWeight: '800',
    letterSpacing: 1.3,
    marginTop: 7,
  },
  verticalRule: { backgroundColor: color.line, height: 60, width: StyleSheet.hairlineWidth },
  incompleteNotice: {
    backgroundColor: color.accentWash,
    borderColor: color.lineBright,
    borderRadius: radius.card,
    borderWidth: 1,
    marginTop: space.md,
    padding: space.md,
  },
  syncNotice: {
    alignItems: 'center',
    backgroundColor: color.surface,
    borderColor: color.line,
    borderRadius: radius.card,
    borderWidth: 1,
    flexDirection: 'row',
    marginTop: space.md,
    padding: space.md,
  },
  syncPulse: {
    backgroundColor: color.accent,
    borderRadius: 5,
    height: 10,
    marginRight: 12,
    shadowColor: color.accent,
    shadowOpacity: 0.8,
    shadowRadius: 8,
    width: 10,
  },
  syncCopy: { flex: 1 },
  syncTitle: { color: color.accent, fontSize: 9, fontWeight: '800', letterSpacing: 1.3 },
  syncBody: { color: color.mutedText, fontSize: 12, lineHeight: 17, marginTop: 5 },
  failedNotice: {
    backgroundColor: color.backgroundLifted,
    borderColor: color.faintText,
    borderRadius: radius.card,
    borderWidth: 1,
    marginTop: space.md,
    padding: space.md,
  },
  failedCode: { color: color.text, fontSize: 9, fontWeight: '800', letterSpacing: 1.4 },
  failedBody: { color: color.mutedText, fontSize: 12, lineHeight: 18, marginTop: 7 },
  failedFoot: { color: color.faintText, fontSize: 10, lineHeight: 15, marginTop: 7 },
  incompleteCode: { color: color.accent, fontSize: 9, fontWeight: '800', letterSpacing: 1.5 },
  incompleteBody: { color: color.mutedText, fontSize: 12, lineHeight: 18, marginTop: 7 },
  sectionLabel: {
    color: color.faintText,
    fontSize: 10,
    fontWeight: '800',
    letterSpacing: 1.6,
    marginBottom: 9,
    marginTop: space.lg,
  },
  emptyCard: {
    alignItems: 'center',
    backgroundColor: color.surface,
    borderColor: color.line,
    borderRadius: radius.card,
    borderWidth: 1,
    flexDirection: 'row',
    padding: space.md,
  },
  emptyIcon: {
    alignItems: 'center',
    backgroundColor: color.accentWash,
    borderRadius: 22,
    height: 54,
    justifyContent: 'center',
    marginRight: 14,
    width: 54,
  },
  emptyCopy: { flex: 1 },
  emptyTitle: { color: color.text, fontSize: 16, fontWeight: '700' },
  emptyBody: { color: color.mutedText, fontSize: 13, lineHeight: 18, marginTop: 4 },
  sessionList: { gap: 10 },
  sessionCard: {
    backgroundColor: color.surface,
    borderColor: color.line,
    borderRadius: radius.card,
    borderWidth: 1,
    overflow: 'hidden',
    padding: space.md,
  },
  sessionRail: { alignItems: 'center', flexDirection: 'row' },
  outcomeMark: { backgroundColor: color.accent, borderRadius: 3, height: 6, width: 6 },
  outcomeMarkCancelled: { backgroundColor: color.faintText },
  outcomeText: {
    color: color.accent,
    fontSize: 8,
    fontWeight: '800',
    letterSpacing: 1.4,
    marginLeft: 7,
  },
  outcomeTextCancelled: { color: color.mutedText },
  sequence: {
    color: color.faintText,
    fontSize: 9,
    fontVariant: ['tabular-nums'],
    marginLeft: 'auto',
  },
  sessionBody: { alignItems: 'center', flexDirection: 'row', paddingVertical: 14 },
  sessionCopy: { flex: 1 },
  sessionTitle: { color: color.text, fontSize: 16, fontWeight: '700' },
  sessionTime: { color: color.mutedText, fontSize: 11, lineHeight: 16, marginTop: 4 },
  sessionDuration: {
    color: color.text,
    fontSize: 22,
    fontVariant: ['tabular-nums'],
    fontWeight: '300',
    marginLeft: 12,
  },
  durationTrack: { backgroundColor: color.line, height: 2 },
  durationFill: { backgroundColor: color.accent, height: 2 },
  durationFillCancelled: { backgroundColor: color.faintText },
  privacyNote: {
    alignItems: 'center',
    flexDirection: 'row',
    gap: 8,
    justifyContent: 'center',
    paddingTop: space.lg,
  },
  privacyText: { color: color.faintText, flex: 1, fontSize: 11, lineHeight: 16 },
  privacyFallback: { color: color.faintText, fontSize: 12 },
});
