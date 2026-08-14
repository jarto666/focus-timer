import { SymbolView } from 'expo-symbols';
import { StyleSheet, Text, View } from 'react-native';

import { Page } from '@/ui/page';
import { Sigil } from '@/ui/sigil';
import { color, radius, space } from '@/ui/theme';

export default function HistoryScreen() {
  return (
    <Page>
      <View style={styles.header}>
        <Text style={styles.eyebrow}>ON THIS IPHONE</Text>
        <Text accessibilityRole="header" style={styles.title}>
          Session history
        </Text>
        <Text style={styles.body}>Completed and cancelled sessions stay available offline.</Text>
      </View>

      <View style={styles.summaryRow}>
        <View style={styles.summaryCell}>
          <Text style={styles.summaryValue}>0</Text>
          <Text style={styles.summaryLabel}>SESSIONS</Text>
        </View>
        <View style={styles.verticalRule} />
        <View style={styles.summaryCell}>
          <Text style={styles.summaryValue}>0m</Text>
          <Text style={styles.summaryLabel}>FOCUS TIME</Text>
        </View>
      </View>

      <Text style={styles.sectionLabel}>RECENT</Text>
      <View style={styles.emptyCard}>
        <View style={styles.emptyIcon}>
          <Sigil name="norns" size={38} />
        </View>
        <View style={styles.emptyCopy}>
          <Text style={styles.emptyTitle}>Nothing here yet</Text>
          <Text style={styles.emptyBody}>
            Your first synchronized timer session will appear here.
          </Text>
        </View>
      </View>

      <View style={styles.privacyNote}>
        <SymbolView
          fallback={<Text style={styles.privacyFallback}>✓</Text>}
          name="iphone.gen3"
          size={16}
          tintColor={color.faintText}
        />
        <Text style={styles.privacyText}>History is stored locally on this iPhone.</Text>
      </View>
    </Page>
  );
}

const styles = StyleSheet.create({
  header: {
    paddingBottom: space.xl,
    paddingTop: space.lg,
  },
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
  body: {
    color: color.mutedText,
    fontSize: 15,
    lineHeight: 22,
    marginTop: space.sm,
  },
  summaryRow: {
    alignItems: 'center',
    backgroundColor: color.backgroundLifted,
    borderColor: color.line,
    borderRadius: radius.panel,
    borderWidth: 1,
    flexDirection: 'row',
    minHeight: 126,
  },
  summaryCell: {
    alignItems: 'center',
    flex: 1,
  },
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
  verticalRule: {
    backgroundColor: color.line,
    height: 60,
    width: StyleSheet.hairlineWidth,
  },
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
  emptyCopy: {
    flex: 1,
  },
  emptyTitle: {
    color: color.text,
    fontSize: 16,
    fontWeight: '700',
  },
  emptyBody: {
    color: color.mutedText,
    fontSize: 13,
    lineHeight: 18,
    marginTop: 4,
  },
  privacyNote: {
    alignItems: 'center',
    flexDirection: 'row',
    gap: 8,
    justifyContent: 'center',
    paddingTop: space.lg,
  },
  privacyText: {
    color: color.faintText,
    fontSize: 11,
  },
  privacyFallback: {
    color: color.faintText,
    fontSize: 12,
  },
});
