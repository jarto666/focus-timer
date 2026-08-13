import { StyleSheet, Text, View } from 'react-native';

import { Page } from '@/ui/page';
import { color, radius, space } from '@/ui/theme';

export default function DeviceScreen() {
  return (
    <Page>
      <View style={styles.header}>
        <Text style={styles.kicker}>NEARBY HARDWARE</Text>
        <Text accessibilityRole="header" style={styles.title}>
          Find your timer.
        </Text>
        <Text style={styles.body}>
          Discovery will appear here when the transport-neutral client is connected to the mock and
          BLE adapters.
        </Text>
      </View>

      <View style={styles.emptyCard}>
        <View style={styles.radar}>
          <View style={styles.radarCore} />
        </View>
        <Text style={styles.emptyTitle}>Waiting for the connection layer</Text>
        <Text style={styles.emptyBody}>
          The screen already owns no Bluetooth state. It will receive the same device model from
          either backend.
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
  },
  body: {
    color: color.mutedInk,
    fontSize: 17,
    lineHeight: 26,
    marginTop: space.md,
  },
  emptyCard: {
    alignItems: 'center',
    backgroundColor: color.raised,
    borderColor: color.line,
    borderRadius: radius.card,
    borderWidth: 1,
    padding: space.xl,
  },
  radar: {
    alignItems: 'center',
    borderColor: color.signalSoft,
    borderRadius: 55,
    borderWidth: 14,
    height: 110,
    justifyContent: 'center',
    marginBottom: space.lg,
    width: 110,
  },
  radarCore: {
    backgroundColor: color.signal,
    borderRadius: 16,
    height: 32,
    width: 32,
  },
  emptyTitle: {
    color: color.ink,
    fontSize: 20,
    fontWeight: '800',
    textAlign: 'center',
  },
  emptyBody: {
    color: color.mutedInk,
    fontSize: 15,
    lineHeight: 22,
    marginTop: space.sm,
    textAlign: 'center',
  },
});
