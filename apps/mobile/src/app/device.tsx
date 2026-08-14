import { StyleSheet, Text, View } from 'react-native';

import { Page } from '@/ui/page';
import { Sigil } from '@/ui/sigil';
import { color, radius, space } from '@/ui/theme';

export default function DeviceScreen() {
  return (
    <Page>
      <View style={styles.header}>
        <Text style={styles.eyebrow}>BLUETOOTH SETUP</Text>
        <Text accessibilityRole="header" style={styles.title}>
          Bring your timer close.
        </Text>
        <Text style={styles.body}>
          Your phone will look for a Focus Timer nearby. Physical sessions keep working with or
          without this connection.
        </Text>
      </View>

      <View style={styles.scanPanel}>
        <View style={styles.radarStage}>
          <View style={styles.radarOuter} />
          <View style={styles.radarMiddle} />
          <View style={styles.radarCore}>
            <Sigil name="bifrost" size={42} />
          </View>
          <View style={styles.beacon} />
        </View>
        <Text style={styles.scanTitle}>Discovery is not enabled yet</Text>
        <Text style={styles.scanBody}>
          This Development Build proves the native iPhone toolchain. BLE discovery is the next
          device milestone.
        </Text>
      </View>

      <Text style={styles.sectionLabel}>CONNECTION</Text>
      <View style={styles.detailCard}>
        <View style={styles.detailRow}>
          <Text style={styles.detailLabel}>Bluetooth transport</Text>
          <Text style={styles.detailValue}>Not configured</Text>
        </View>
        <View style={styles.rule} />
        <View style={styles.detailRow}>
          <Text style={styles.detailLabel}>Timer operation</Text>
          <Text style={styles.readyValue}>Independent</Text>
        </View>
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
  scanPanel: {
    alignItems: 'center',
    backgroundColor: color.backgroundLifted,
    borderColor: color.line,
    borderRadius: radius.panel,
    borderWidth: 1,
    padding: space.lg,
  },
  radarStage: {
    alignItems: 'center',
    height: 190,
    justifyContent: 'center',
    marginBottom: space.md,
    width: 190,
  },
  radarOuter: {
    position: 'absolute',
    borderColor: color.line,
    borderRadius: 92,
    borderWidth: 1,
    height: 184,
    width: 184,
  },
  radarMiddle: {
    position: 'absolute',
    backgroundColor: color.accentWash,
    borderColor: color.lineBright,
    borderRadius: 62,
    borderWidth: 1,
    height: 124,
    shadowColor: color.accent,
    shadowOpacity: 0.2,
    shadowRadius: 30,
    width: 124,
  },
  radarCore: {
    alignItems: 'center',
    backgroundColor: color.accent,
    borderColor: color.accentBright,
    borderRadius: 36,
    borderWidth: 1,
    height: 72,
    justifyContent: 'center',
    width: 72,
  },
  beacon: {
    position: 'absolute',
    right: 18,
    top: 55,
    backgroundColor: color.accentBright,
    borderRadius: 4,
    height: 7,
    shadowColor: color.accent,
    shadowOpacity: 0.95,
    shadowRadius: 10,
    width: 7,
  },
  scanTitle: {
    color: color.text,
    fontSize: 18,
    fontWeight: '700',
    textAlign: 'center',
  },
  scanBody: {
    color: color.mutedText,
    fontSize: 13,
    lineHeight: 19,
    marginTop: 8,
    textAlign: 'center',
  },
  sectionLabel: {
    color: color.faintText,
    fontSize: 10,
    fontWeight: '800',
    letterSpacing: 1.6,
    marginBottom: 9,
    marginTop: space.lg,
  },
  detailCard: {
    backgroundColor: color.surface,
    borderColor: color.line,
    borderRadius: radius.card,
    borderWidth: 1,
    paddingHorizontal: space.md,
  },
  detailRow: {
    alignItems: 'center',
    flexDirection: 'row',
    justifyContent: 'space-between',
    minHeight: 52,
  },
  detailLabel: {
    color: color.text,
    fontSize: 14,
    fontWeight: '600',
  },
  detailValue: {
    color: color.mutedText,
    fontSize: 13,
  },
  readyValue: {
    color: color.accent,
    fontSize: 13,
    fontWeight: '700',
  },
  rule: {
    backgroundColor: color.line,
    height: StyleSheet.hairlineWidth,
  },
});
