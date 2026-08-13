import { StyleSheet, Text, View } from 'react-native';

import { Page } from '@/ui/page';
import { color, radius, space } from '@/ui/theme';

export default function HistoryScreen() {
  return (
    <Page>
      <View style={styles.header}>
        <Text style={styles.kicker}>ON THIS IPHONE</Text>
        <Text accessibilityRole="header" style={styles.title}>
          Quiet proof of work.
        </Text>
      </View>

      <View style={styles.emptyCard}>
        <Text style={styles.zero}>00</Text>
        <View style={styles.rule} />
        <Text style={styles.emptyTitle}>No sessions stored yet</Text>
        <Text style={styles.emptyBody}>
          Completed and cancelled sessions will remain readable here even when your timer is out of
          range.
        </Text>
      </View>
    </Page>
  );
}

const styles = StyleSheet.create({
  header: {
    paddingBottom: space.xl,
    paddingTop: space.xl,
  },
  kicker: {
    color: color.signal,
    fontFamily: 'Courier',
    fontSize: 12,
    fontWeight: '700',
    letterSpacing: 1.4,
    marginBottom: space.md,
  },
  title: {
    color: color.ink,
    fontFamily: 'Georgia',
    fontSize: 48,
    letterSpacing: -2,
    lineHeight: 52,
    maxWidth: 330,
  },
  emptyCard: {
    backgroundColor: color.ink,
    borderRadius: radius.card,
    padding: space.lg,
  },
  zero: {
    color: color.signal,
    fontFamily: 'Georgia',
    fontSize: 80,
    letterSpacing: -4,
    lineHeight: 88,
  },
  rule: {
    backgroundColor: '#3B3A34',
    height: 1,
    marginBottom: space.lg,
  },
  emptyTitle: {
    color: color.raised,
    fontSize: 20,
    fontWeight: '800',
  },
  emptyBody: {
    color: '#B8B5AA',
    fontSize: 15,
    lineHeight: 23,
    marginTop: space.sm,
  },
});
