import type { ConnectionPhase, DeviceCandidate } from '@focus-timer/device-client';
import { Pressable, StyleSheet, Text, View } from 'react-native';

import { useCompanionRuntime } from '@/application/runtime-provider';
import { Page } from '@/ui/page';
import { Sigil } from '@/ui/sigil';
import { color, radius, space } from '@/ui/theme';

type ConnectionPresentation = Readonly<{
  eyebrow: string;
  title: string;
  body: string;
  signal: string;
}>;

function presentationFor(phase: ConnectionPhase): ConnectionPresentation {
  switch (phase) {
    case 'disconnected':
      return {
        eyebrow: 'DEVICE LINK',
        title: 'Find your timer.',
        body: 'Keep it nearby and powered. Sessions continue to work even when Muninn is disconnected.',
        signal: 'STANDING BY',
      };
    case 'scanning':
      return {
        eyebrow: 'NEARBY SIGNAL',
        title: 'Timer found.',
        body: 'Confirm the nearby Focus Timer to read its status and synchronize finished sessions.',
        signal: 'DISCOVERED',
      };
    case 'connecting':
    case 'handshaking':
      return {
        eyebrow: 'SECURE LINK',
        title: 'Verifying the timer.',
        body: 'Muninn is checking device identity and protocol compatibility before reading data.',
        signal: 'LINKING',
      };
    case 'ready':
      return {
        eyebrow: 'DEVICE LINK',
        title: 'Timer connected.',
        body: 'The hardware remains authoritative. Muninn only reads status and remembers finished sessions.',
        signal: 'READY',
      };
    case 'unavailable':
      return {
        eyebrow: 'NO SIGNAL',
        title: 'Timer unavailable.',
        body: 'Bluetooth is unavailable or the selected development device is powered off.',
        signal: 'OFFLINE',
      };
    case 'permission-denied':
      return {
        eyebrow: 'ACCESS NEEDED',
        title: 'Bluetooth is blocked.',
        body: 'Allow Bluetooth access in iPhone Settings before searching again.',
        signal: 'BLOCKED',
      };
    case 'incompatible':
      return {
        eyebrow: 'PROTOCOL CHECK',
        title: 'Update required.',
        body: 'This app will not read a timer whose protocol it cannot interpret safely.',
        signal: 'INCOMPATIBLE',
      };
    case 'retryable-error':
      return {
        eyebrow: 'LINK INTERRUPTED',
        title: 'Connection lost.',
        body: 'Nothing local was discarded. Search again to resume from the last known cursor.',
        signal: 'RETRY',
      };
  }
}

function CandidateCard({
  candidate,
  onConnect,
}: Readonly<{ candidate: DeviceCandidate; onConnect(): void }>) {
  return (
    <View style={styles.candidateCard}>
      <View style={styles.candidateSignal}>
        <View style={styles.candidateSignalCore} />
      </View>
      <View style={styles.candidateCopy}>
        <Text style={styles.candidateName}>{candidate.productName ?? 'Focus Timer'}</Text>
        <Text style={styles.candidateMeta}>Nearby · signal {candidate.rssi ?? '—'} dBm</Text>
      </View>
      <Pressable accessibilityRole="button" onPress={onConnect} style={styles.compactButton}>
        <Text style={styles.compactButtonText}>Connect</Text>
      </Pressable>
    </View>
  );
}

export default function DeviceScreen() {
  const {
    connection,
    status,
    selectedScenario,
    developmentScenarios,
    startScan,
    connect,
    disconnect,
    selectScenario,
  } = useCompanionRuntime();
  const presentation = presentationFor(connection.phase);
  const busy = connection.phase === 'connecting' || connection.phase === 'handshaking';
  const candidates = connection.phase === 'scanning' ? connection.candidates : [];
  const canSearch =
    connection.phase === 'disconnected' ||
    connection.phase === 'retryable-error' ||
    connection.phase === 'incompatible' ||
    connection.phase === 'permission-denied' ||
    (connection.phase === 'unavailable' && connection.reason === 'powered-off');

  return (
    <Page>
      <View style={styles.header}>
        <Text style={styles.eyebrow}>{presentation.eyebrow}</Text>
        <Text accessibilityRole="header" style={styles.title}>
          {presentation.title}
        </Text>
        <Text style={styles.body}>{presentation.body}</Text>
      </View>

      <View style={styles.signalPanel}>
        <View style={styles.signalRail}>
          <View
            style={[styles.signalMark, connection.phase === 'ready' && styles.signalMarkReady]}
          />
          <Text style={styles.signalCode}>{presentation.signal}</Text>
        </View>
        <View style={styles.radarStage}>
          <View style={styles.radarOuter} />
          <View style={[styles.radarMiddle, connection.phase === 'ready' && styles.radarReady]} />
          <View style={styles.radarCore}>
            <Sigil name="bifrost" size={42} />
          </View>
          <View style={styles.axisHorizontal} />
          <View style={styles.axisVertical} />
        </View>

        {candidates.map((candidate) => (
          <CandidateCard
            candidate={candidate}
            key={candidate.transportId}
            onConnect={() => void connect(candidate.transportId)}
          />
        ))}

        {connection.phase === 'scanning' && candidates.length === 0 ? (
          <Text style={styles.waitingText}>Listening for nearby hardware…</Text>
        ) : null}

        {canSearch ? (
          <Pressable
            accessibilityRole="button"
            onPress={() => void startScan()}
            style={styles.primaryButton}
          >
            <Text style={styles.primaryButtonText}>
              {connection.phase === 'disconnected' ? 'Search nearby' : 'Search again'}
            </Text>
          </Pressable>
        ) : null}

        {busy ? <Text style={styles.waitingText}>Checking device identity…</Text> : null}
      </View>

      {connection.phase === 'ready' ? (
        <>
          <Text style={styles.sectionLabel}>READ-ONLY STATUS</Text>
          <View style={styles.detailCard}>
            <View style={styles.detailRow}>
              <Text style={styles.detailLabel}>Device</Text>
              <Text style={styles.detailValue}>
                {connection.device.productName ?? connection.device.deviceId}
              </Text>
            </View>
            <View style={styles.rule} />
            <View style={styles.detailRow}>
              <Text style={styles.detailLabel}>Firmware</Text>
              <Text style={styles.detailValue}>{connection.device.firmwareVersion}</Text>
            </View>
            <View style={styles.rule} />
            <View style={styles.detailRow}>
              <Text style={styles.detailLabel}>Selected preset</Text>
              <Text style={styles.readyValue}>{status?.presetName ?? 'Unknown'}</Text>
            </View>
            <View style={styles.rule} />
            <View style={styles.detailRow}>
              <Text style={styles.detailLabel}>Timer operation</Text>
              <Text style={styles.readyValue}>Independent</Text>
            </View>
          </View>
          <Pressable
            accessibilityRole="button"
            onPress={() => void disconnect()}
            style={styles.secondaryButton}
          >
            <Text style={styles.secondaryButtonText}>Disconnect</Text>
          </Pressable>
        </>
      ) : null}

      {connection.phase === 'incompatible' ? (
        <View style={styles.notice}>
          <Text style={styles.noticeTitle}>Supported protocol: {connection.supportedMajor}.x</Text>
          <Text style={styles.noticeBody}>Timer reported: {connection.receivedMajor}.x</Text>
        </View>
      ) : null}

      {connection.phase === 'retryable-error' ? (
        <View style={styles.notice}>
          <Text style={styles.noticeTitle}>{connection.error.code.replaceAll('-', ' ')}</Text>
          <Text style={styles.noticeBody}>{connection.error.message}</Text>
        </View>
      ) : null}

      {developmentScenarios.length > 0 ? (
        <View style={styles.lab}>
          <Text style={styles.sectionLabel}>DEVELOPMENT SCENARIO</Text>
          <Text style={styles.labBody}>
            Swap deterministic device behavior, then search again. This control disappears from
            production BLE builds.
          </Text>
          <View style={styles.scenarioGrid}>
            {developmentScenarios.map((scenario) => {
              const selected = scenario.id === selectedScenario;
              return (
                <Pressable
                  accessibilityRole="button"
                  key={scenario.id}
                  onPress={() => selectScenario(scenario.id)}
                  style={[styles.scenarioChip, selected && styles.scenarioChipSelected]}
                >
                  <Text style={[styles.scenarioText, selected && styles.scenarioTextSelected]}>
                    {scenario.label}
                  </Text>
                </Pressable>
              );
            })}
          </View>
        </View>
      ) : null}
    </Page>
  );
}

const styles = StyleSheet.create({
  header: { paddingBottom: space.lg, paddingTop: space.lg },
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
  signalPanel: {
    alignItems: 'center',
    backgroundColor: color.backgroundLifted,
    borderColor: color.line,
    borderRadius: radius.panel,
    borderWidth: 1,
    overflow: 'hidden',
    padding: space.md,
  },
  signalRail: {
    alignItems: 'center',
    alignSelf: 'stretch',
    flexDirection: 'row',
    gap: 8,
  },
  signalMark: { backgroundColor: color.faintText, borderRadius: 3, height: 6, width: 6 },
  signalMarkReady: {
    backgroundColor: color.accent,
    shadowColor: color.accent,
    shadowOpacity: 1,
    shadowRadius: 7,
  },
  signalCode: { color: color.faintText, fontSize: 9, fontWeight: '800', letterSpacing: 1.7 },
  radarStage: {
    alignItems: 'center',
    height: 190,
    justifyContent: 'center',
    width: 190,
  },
  radarOuter: {
    position: 'absolute',
    borderColor: color.line,
    borderRadius: 86,
    borderWidth: 1,
    height: 172,
    width: 172,
  },
  radarMiddle: {
    position: 'absolute',
    backgroundColor: color.accentWash,
    borderColor: color.lineBright,
    borderRadius: 58,
    borderWidth: 1,
    height: 116,
    width: 116,
  },
  radarReady: { shadowColor: color.accent, shadowOpacity: 0.4, shadowRadius: 30 },
  radarCore: {
    alignItems: 'center',
    backgroundColor: color.accent,
    borderRadius: 34,
    height: 68,
    justifyContent: 'center',
    width: 68,
    zIndex: 1,
  },
  axisHorizontal: { position: 'absolute', backgroundColor: color.line, height: 1, width: 190 },
  axisVertical: { position: 'absolute', backgroundColor: color.line, height: 190, width: 1 },
  candidateCard: {
    alignItems: 'center',
    alignSelf: 'stretch',
    backgroundColor: color.surface,
    borderColor: color.lineBright,
    borderRadius: radius.card,
    borderWidth: 1,
    flexDirection: 'row',
    padding: 12,
  },
  candidateSignal: {
    alignItems: 'center',
    borderColor: color.accent,
    borderRadius: 18,
    borderWidth: 1,
    height: 36,
    justifyContent: 'center',
    width: 36,
  },
  candidateSignalCore: {
    backgroundColor: color.accentBright,
    borderRadius: 3,
    height: 6,
    width: 6,
  },
  candidateCopy: { flex: 1, marginHorizontal: 11 },
  candidateName: { color: color.text, fontSize: 14, fontWeight: '700' },
  candidateMeta: { color: color.mutedText, fontSize: 11, marginTop: 3 },
  compactButton: {
    backgroundColor: color.accent,
    borderRadius: 10,
    paddingHorizontal: 12,
    paddingVertical: 9,
  },
  compactButtonText: { color: color.background, fontSize: 12, fontWeight: '800' },
  primaryButton: {
    alignItems: 'center',
    alignSelf: 'stretch',
    backgroundColor: color.accent,
    borderRadius: radius.control,
    justifyContent: 'center',
    minHeight: 52,
  },
  primaryButtonText: { color: color.background, fontSize: 14, fontWeight: '800' },
  waitingText: { color: color.mutedText, fontSize: 13, paddingBottom: space.sm },
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
  detailLabel: { color: color.text, fontSize: 14, fontWeight: '600' },
  detailValue: { color: color.mutedText, fontSize: 13, maxWidth: '55%', textAlign: 'right' },
  readyValue: { color: color.accent, fontSize: 13, fontWeight: '700' },
  rule: { backgroundColor: color.line, height: StyleSheet.hairlineWidth },
  secondaryButton: {
    alignItems: 'center',
    borderColor: color.lineBright,
    borderRadius: radius.control,
    borderWidth: 1,
    justifyContent: 'center',
    marginTop: space.sm,
    minHeight: 48,
  },
  secondaryButtonText: { color: color.mutedText, fontSize: 13, fontWeight: '700' },
  notice: {
    backgroundColor: color.surface,
    borderColor: color.danger,
    borderRadius: radius.card,
    borderWidth: 1,
    marginTop: space.md,
    padding: space.md,
  },
  noticeTitle: { color: color.text, fontSize: 14, fontWeight: '700', textTransform: 'capitalize' },
  noticeBody: { color: color.mutedText, fontSize: 12, lineHeight: 18, marginTop: 4 },
  lab: { paddingBottom: space.md },
  labBody: { color: color.mutedText, fontSize: 12, lineHeight: 18, marginBottom: space.sm },
  scenarioGrid: { flexDirection: 'row', flexWrap: 'wrap', gap: 8 },
  scenarioChip: {
    borderColor: color.line,
    borderRadius: radius.pill,
    borderWidth: 1,
    paddingHorizontal: 12,
    paddingVertical: 8,
  },
  scenarioChipSelected: { backgroundColor: color.accentWash, borderColor: color.accent },
  scenarioText: { color: color.mutedText, fontSize: 11, fontWeight: '700' },
  scenarioTextSelected: { color: color.accentBright },
});
