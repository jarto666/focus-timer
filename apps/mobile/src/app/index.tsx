import { Link } from 'expo-router';
import { SymbolView } from 'expo-symbols';
import { Pressable, StyleSheet, Text, View } from 'react-native';

import { useCompanionRuntime } from '@/application/runtime-provider';
import { Page } from '@/ui/page';
import { Sigil } from '@/ui/sigil';
import { color, radius, space } from '@/ui/theme';

function formatTimer(milliseconds: number): string {
  const totalSeconds = Math.max(0, Math.floor(milliseconds / 1_000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
}

function connectionCopy(phase: ReturnType<typeof useCompanionRuntime>['connection']['phase']) {
  switch (phase) {
    case 'ready':
      return {
        eyebrow: 'DEVICE // LINKED',
        title: 'Ready when you are.',
        detail: 'Your physical timer owns the session. Muninn observes and remembers.',
        badge: 'READY',
        dial: 'DEVICE IDLE',
        pairLabel: 'View timer',
        pairDetail: 'Connection and device status',
      };
    case 'scanning':
      return {
        eyebrow: 'DEVICE // SEARCHING',
        title: 'Signal detected.',
        detail: 'A nearby Focus Timer is available to connect.',
        badge: 'SCANNING',
        dial: 'SEARCHING',
        pairLabel: 'Continue setup',
        pairDetail: 'Choose the nearby timer',
      };
    case 'connecting':
    case 'handshaking':
      return {
        eyebrow: 'DEVICE // LINKING',
        title: 'Opening the link.',
        detail: 'Muninn is verifying the nearby timer before reading any data.',
        badge: 'LINKING',
        dial: 'VERIFYING',
        pairLabel: 'View connection',
        pairDetail: 'Handshake in progress',
      };
    case 'unavailable':
      return {
        eyebrow: 'DEVICE // UNAVAILABLE',
        title: 'Timer out of reach.',
        detail: 'Local history remains available while the device is offline.',
        badge: 'OFFLINE',
        dial: 'NO SIGNAL',
        pairLabel: 'Try again',
        pairDetail: 'Check the nearby timer',
      };
    case 'permission-denied':
      return {
        eyebrow: 'DEVICE // BLOCKED',
        title: 'Bluetooth access needed.',
        detail: 'Muninn cannot discover hardware until permission is restored.',
        badge: 'BLOCKED',
        dial: 'NO ACCESS',
        pairLabel: 'Review connection',
        pairDetail: 'See the recovery path',
      };
    case 'incompatible':
      return {
        eyebrow: 'DEVICE // UNSUPPORTED',
        title: 'Update required.',
        detail: 'The timer speaks a protocol version this build cannot read safely.',
        badge: 'VERSION',
        dial: 'READ DISABLED',
        pairLabel: 'View details',
        pairDetail: 'Inspect compatibility',
      };
    case 'retryable-error':
      return {
        eyebrow: 'DEVICE // INTERRUPTED',
        title: 'The link broke.',
        detail: 'Nothing local was discarded. You can retry the connection.',
        badge: 'RETRY',
        dial: 'LINK LOST',
        pairLabel: 'Retry connection',
        pairDetail: 'Resume from saved progress',
      };
    case 'disconnected':
      return {
        eyebrow: 'DEVICE // UNPAIRED',
        title: 'Unpaired.',
        detail: 'No Focus Timer is linked to this iPhone.',
        badge: 'OFFLINE',
        dial: 'AWAITING DEVICE',
        pairLabel: 'Pair a timer',
        pairDetail: 'Search nearby hardware',
      };
  }
}

export default function HomeScreen() {
  const { connection, history, status } = useCompanionRuntime();
  const copy = connectionCopy(connection.phase);
  const ready = connection.phase === 'ready';
  const entryCount = String(history.entries.length).padStart(2, '0');

  return (
    <Page>
      <View style={styles.heading}>
        <Text style={styles.eyebrow}>{copy.eyebrow}</Text>
        <Text accessibilityRole="header" style={styles.title}>
          {copy.title}
        </Text>
        <Text style={styles.headingDetail}>{copy.detail}</Text>
      </View>

      <View style={styles.timerPanel}>
        <View style={styles.panelHeader}>
          <View>
            <Text style={styles.deviceName}>Focus Timer</Text>
            <Text style={styles.deviceMeta}>
              {ready ? 'Read-only device state' : 'No active device link'}
            </Text>
          </View>
          <View style={styles.statusBadge}>
            <View style={[styles.offlineDot, ready && styles.readyDot]} />
            <Text style={[styles.statusLabel, ready && styles.readyStatusLabel]}>{copy.badge}</Text>
          </View>
        </View>

        <View
          accessible
          accessibilityLabel={
            status === null
              ? `Timer ${copy.dial.toLowerCase()}`
              : `${status.presetName}, ${formatTimer(status.remainingDurationMs)} remaining`
          }
          style={styles.dialStage}
        >
          <View style={[styles.tick, styles.tickTop]} />
          <View style={[styles.tick, styles.tickRight]} />
          <View style={[styles.tick, styles.tickBottom]} />
          <View style={[styles.tick, styles.tickLeft]} />
          <View style={styles.outerDial}>
            <View style={styles.arc} />
            <View style={styles.innerDial}>
              <Sigil name="time" size={28} />
              <Text style={styles.time}>
                {status === null ? '--:--' : formatTimer(status.remainingDurationMs)}
              </Text>
              <Text style={styles.dialLabel}>{copy.dial}</Text>
            </View>
          </View>
        </View>

        <View style={styles.panelFooter}>
          <View style={styles.footerGlyph}>
            <View style={styles.footerGlyphCore} />
          </View>
          <Text style={styles.panelMessage}>
            {ready
              ? 'Status is read-only. Sessions still start, pause, and finish on the physical timer.'
              : 'Pair your physical timer to sync sessions and see its live state.'}
          </Text>
        </View>
      </View>

      <Link asChild href="/device">
        <Pressable accessibilityRole="button" style={styles.pairButton}>
          <View style={styles.pairRune}>
            <Sigil name="bifrost" size={34} />
          </View>
          <View style={styles.pairCopy}>
            <Text style={styles.pairLabel}>{copy.pairLabel}</Text>
            <Text style={styles.pairDetail}>{copy.pairDetail}</Text>
          </View>
          <SymbolView
            fallback={<Text style={styles.pairChevronFallback}>›</Text>}
            name="chevron.right"
            size={14}
            tintColor={color.background}
            weight="semibold"
          />
        </Pressable>
      </Link>

      <Link asChild href="/history">
        <Pressable accessibilityRole="button" style={styles.ledger}>
          <View style={styles.ledgerRail}>
            <Text style={styles.ledgerCode}>SESSION LEDGER</Text>
            <View style={styles.localBadge}>
              <View style={styles.localBadgeDot} />
              <Text style={styles.localBadgeText}>LOCAL</Text>
            </View>
          </View>

          <View style={styles.ledgerBody}>
            <View style={styles.countBlock}>
              <Text style={styles.ledgerCount}>{entryCount}</Text>
              <Text style={styles.ledgerUnit}>ENTRIES</Text>
            </View>
            <View style={styles.ledgerRule} />
            <View style={styles.ledgerCopy}>
              <Text style={styles.ledgerTitle}>Your focus archive</Text>
              <Text style={styles.ledgerDetail}>Synced sessions stay on this iPhone.</Text>
            </View>
            <View style={styles.ledgerRune}>
              <Sigil name="norns" size={34} />
            </View>
          </View>

          <View style={styles.ledgerTrack}>
            <View style={styles.ledgerTrackActive} />
          </View>
        </Pressable>
      </Link>

      <View style={styles.localRow}>
        <SymbolView
          fallback={<Text style={styles.localFallback}>✓</Text>}
          name="lock.shield"
          size={15}
          tintColor={color.faintText}
        />
        <Text style={styles.localText}>Local-first · no account · no cloud required</Text>
      </View>
    </Page>
  );
}

const styles = StyleSheet.create({
  heading: {
    paddingBottom: space.lg,
    paddingTop: space.xl,
  },
  eyebrow: {
    color: color.accent,
    fontSize: 10,
    fontWeight: '800',
    letterSpacing: 2,
    marginBottom: 8,
  },
  title: {
    color: color.text,
    fontSize: 31,
    fontWeight: '700',
    letterSpacing: -1,
    lineHeight: 36,
  },
  headingDetail: {
    color: color.mutedText,
    fontSize: 13,
    marginTop: 7,
  },
  timerPanel: {
    backgroundColor: color.backgroundLifted,
    borderColor: color.line,
    borderRadius: radius.panel,
    borderWidth: 1,
    overflow: 'hidden',
    padding: space.md,
    shadowColor: '#000000',
    shadowOffset: { width: 0, height: 16 },
    shadowOpacity: 0.34,
    shadowRadius: 28,
  },
  panelHeader: {
    alignItems: 'center',
    flexDirection: 'row',
    justifyContent: 'space-between',
  },
  deviceName: {
    color: color.text,
    fontSize: 15,
    fontWeight: '700',
  },
  deviceMeta: {
    color: color.mutedText,
    fontSize: 12,
    marginTop: 3,
  },
  statusBadge: {
    alignItems: 'center',
    backgroundColor: color.surface,
    borderColor: color.line,
    borderRadius: radius.pill,
    borderWidth: 1,
    flexDirection: 'row',
    gap: 6,
    paddingHorizontal: 10,
    paddingVertical: 7,
  },
  offlineDot: {
    backgroundColor: color.faintText,
    borderRadius: 3,
    height: 6,
    width: 6,
  },
  readyDot: {
    backgroundColor: color.accent,
    shadowColor: color.accent,
    shadowOpacity: 0.9,
    shadowRadius: 6,
  },
  statusLabel: {
    color: color.mutedText,
    fontSize: 9,
    fontWeight: '800',
    letterSpacing: 1.1,
  },
  readyStatusLabel: {
    color: color.accent,
  },
  dialStage: {
    alignItems: 'center',
    height: 240,
    justifyContent: 'center',
    marginVertical: 4,
  },
  outerDial: {
    alignItems: 'center',
    borderColor: color.lineBright,
    borderRadius: 104,
    borderWidth: 1,
    height: 208,
    justifyContent: 'center',
    shadowColor: color.accent,
    shadowOpacity: 0.12,
    shadowRadius: 24,
    width: 208,
  },
  arc: {
    position: 'absolute',
    borderBottomColor: 'transparent',
    borderColor: color.accent,
    borderLeftColor: 'transparent',
    borderRadius: 104,
    borderWidth: 2,
    height: 208,
    opacity: 0.72,
    transform: [{ rotate: '-32deg' }],
    width: 208,
  },
  innerDial: {
    alignItems: 'center',
    backgroundColor: color.surface,
    borderColor: color.line,
    borderRadius: 84,
    borderWidth: 1,
    height: 168,
    justifyContent: 'center',
    width: 168,
  },
  time: {
    color: color.text,
    fontSize: 39,
    fontVariant: ['tabular-nums'],
    fontWeight: '300',
    letterSpacing: 1.5,
    lineHeight: 49,
    marginTop: 5,
  },
  dialLabel: {
    color: color.faintText,
    fontSize: 8,
    fontWeight: '800',
    letterSpacing: 1.6,
  },
  tick: {
    position: 'absolute',
    backgroundColor: color.lineBright,
    borderRadius: 1,
    height: 8,
    width: 2,
  },
  tickTop: { top: 8 },
  tickRight: { right: 42, transform: [{ rotate: '90deg' }] },
  tickBottom: { bottom: 8 },
  tickLeft: { left: 42, transform: [{ rotate: '90deg' }] },
  panelFooter: {
    alignItems: 'center',
    backgroundColor: color.accentWash,
    borderColor: color.line,
    borderRadius: radius.card,
    borderWidth: 1,
    flexDirection: 'row',
    padding: 12,
  },
  footerGlyph: {
    alignItems: 'center',
    borderColor: color.accent,
    borderWidth: 1,
    height: 18,
    justifyContent: 'center',
    marginLeft: 2,
    marginRight: 12,
    transform: [{ rotate: '45deg' }],
    width: 18,
  },
  footerGlyphCore: {
    backgroundColor: color.accentBright,
    height: 4,
    width: 4,
  },
  panelMessage: {
    color: color.mutedText,
    flex: 1,
    fontSize: 12,
    lineHeight: 17,
  },
  pairButton: {
    alignItems: 'center',
    backgroundColor: color.accent,
    borderRadius: radius.control,
    flexDirection: 'row',
    marginTop: space.md,
    minHeight: 68,
    paddingHorizontal: space.md,
    shadowColor: color.accent,
    shadowOffset: { width: 0, height: 8 },
    shadowOpacity: 0.16,
    shadowRadius: 18,
  },
  pairRune: {
    alignItems: 'center',
    backgroundColor: 'rgba(6, 17, 21, 0.12)',
    borderColor: 'rgba(6, 17, 21, 0.16)',
    borderRadius: 10,
    borderWidth: 1,
    height: 40,
    justifyContent: 'center',
    marginRight: 12,
    width: 40,
  },
  pairCopy: {
    flex: 1,
  },
  pairLabel: {
    color: color.background,
    fontSize: 15,
    fontWeight: '800',
  },
  pairDetail: {
    color: 'rgba(6, 17, 21, 0.62)',
    fontSize: 12,
    marginTop: 3,
  },
  pairChevronFallback: {
    color: color.background,
    fontSize: 22,
  },
  ledger: {
    backgroundColor: color.surface,
    borderColor: color.line,
    borderRadius: radius.card,
    borderWidth: 1,
    marginTop: space.sm,
    overflow: 'hidden',
    padding: space.md,
  },
  ledgerRail: {
    alignItems: 'center',
    flexDirection: 'row',
    justifyContent: 'space-between',
  },
  ledgerCode: {
    color: color.faintText,
    fontSize: 9,
    fontWeight: '800',
    letterSpacing: 1.7,
  },
  localBadge: {
    alignItems: 'center',
    flexDirection: 'row',
    gap: 5,
  },
  localBadgeDot: {
    backgroundColor: color.accent,
    borderRadius: 2,
    height: 4,
    shadowColor: color.accent,
    shadowOpacity: 0.9,
    shadowRadius: 6,
    width: 4,
  },
  localBadgeText: {
    color: color.accent,
    fontSize: 8,
    fontWeight: '800',
    letterSpacing: 1.2,
  },
  ledgerBody: {
    alignItems: 'center',
    flexDirection: 'row',
    minHeight: 82,
    paddingTop: 5,
  },
  countBlock: {
    width: 62,
  },
  ledgerCount: {
    color: color.accentBright,
    fontSize: 32,
    fontVariant: ['tabular-nums'],
    fontWeight: '300',
    letterSpacing: -1,
  },
  ledgerUnit: {
    color: color.faintText,
    fontSize: 7,
    fontWeight: '800',
    letterSpacing: 1.2,
    marginTop: -1,
  },
  ledgerRule: {
    backgroundColor: color.line,
    height: 47,
    marginRight: 14,
    width: StyleSheet.hairlineWidth,
  },
  ledgerCopy: {
    flex: 1,
  },
  ledgerTitle: {
    color: color.text,
    fontSize: 15,
    fontWeight: '700',
  },
  ledgerDetail: {
    color: color.mutedText,
    fontSize: 11,
    lineHeight: 16,
    marginTop: 4,
  },
  ledgerRune: {
    alignItems: 'center',
    backgroundColor: color.accentWash,
    borderRadius: 11,
    height: 42,
    justifyContent: 'center',
    marginLeft: 10,
    width: 42,
  },
  ledgerTrack: {
    backgroundColor: color.line,
    height: 1,
    marginTop: 2,
  },
  ledgerTrackActive: {
    backgroundColor: color.accent,
    height: 1,
    opacity: 0.8,
    width: 42,
  },
  localRow: {
    alignItems: 'center',
    flexDirection: 'row',
    gap: 8,
    justifyContent: 'center',
    paddingTop: space.lg,
  },
  localText: {
    color: color.faintText,
    fontSize: 11,
  },
  localFallback: {
    color: color.faintText,
    fontSize: 12,
  },
});
