import { StyleSheet, Text, View } from 'react-native';

import { ActionLink } from '@/ui/action-link';
import { Page } from '@/ui/page';
import { color, radius, space } from '@/ui/theme';
import { Wordmark } from '@/ui/wordmark';

export default function HomeScreen() {
  return (
    <Page>
      <Wordmark />

      <View style={styles.hero}>
        <Text style={styles.kicker}>A PHYSICAL PROMISE</Text>
        <Text accessibilityRole="header" style={styles.title}>
          Time you can hold.
        </Text>
        <Text style={styles.subtitle}>
          The timer stays offline and in charge. Your phone remembers what you finished.
        </Text>
      </View>

      <View style={styles.deviceCard}>
        <View style={styles.cardHeader}>
          <View style={styles.statusDot} />
          <Text style={styles.cardEyebrow}>TIMER STATUS</Text>
        </View>
        <Text style={styles.cardTitle}>No timer connected</Text>
        <Text style={styles.cardBody}>
          Pair one nearby Focus Timer. Sessions remain fully usable before, during, and after setup.
        </Text>
        <ActionLink href="/device" label="Set up timer" />
      </View>

      <View style={styles.historyBlock}>
        <View>
          <Text style={styles.historyCount}>00</Text>
          <Text style={styles.historyCaption}>sessions stored locally</Text>
        </View>
        <ActionLink href="/history" label="Open history" tone="light" />
      </View>
    </Page>
  );
}

const styles = StyleSheet.create({
  hero: {
    paddingBottom: space.xl,
    paddingTop: 76,
  },
  kicker: {
    color: color.signal,
    fontFamily: 'Courier',
    fontSize: 12,
    fontWeight: '700',
    letterSpacing: 1.5,
    marginBottom: space.md,
  },
  title: {
    color: color.ink,
    fontFamily: 'Georgia',
    fontSize: 58,
    letterSpacing: -2.8,
    lineHeight: 58,
    maxWidth: 340,
  },
  subtitle: {
    color: color.mutedInk,
    fontSize: 17,
    lineHeight: 26,
    marginTop: space.lg,
    maxWidth: 350,
  },
  deviceCard: {
    backgroundColor: color.signal,
    borderRadius: radius.card,
    gap: space.md,
    padding: space.lg,
  },
  cardHeader: {
    alignItems: 'center',
    flexDirection: 'row',
    gap: space.sm,
  },
  statusDot: {
    width: 8,
    height: 8,
    borderRadius: 4,
    backgroundColor: color.ink,
  },
  cardEyebrow: {
    color: color.ink,
    fontFamily: 'Courier',
    fontSize: 11,
    fontWeight: '700',
    letterSpacing: 1.2,
  },
  cardTitle: {
    color: color.ink,
    fontSize: 28,
    fontWeight: '800',
    letterSpacing: -0.8,
  },
  cardBody: {
    color: color.ink,
    fontSize: 15,
    lineHeight: 22,
    marginBottom: space.xs,
    opacity: 0.78,
  },
  historyBlock: {
    gap: space.lg,
    paddingTop: space.xl,
  },
  historyCount: {
    color: color.ink,
    fontFamily: 'Georgia',
    fontSize: 64,
    letterSpacing: -3,
    lineHeight: 68,
  },
  historyCaption: {
    color: color.mutedInk,
    fontSize: 15,
  },
});
