import { useMemo } from 'react';
import { Pressable, StyleSheet, Text, TextInput, View } from 'react-native';

import { useCompanionRuntime } from '@/application/runtime-provider';
import type { PresetDraft, PresetSyncState } from '@/application/companion-model';
import { Page } from '@/ui/page';
import { Sigil } from '@/ui/sigil';
import { color, radius, space } from '@/ui/theme';

const syncCopy: Record<PresetSyncState, { label: string; detail: string }> = {
  unavailable: { label: 'OFFLINE DRAFT', detail: 'Connect the timer before sending changes.' },
  loading: { label: 'READING RUNES', detail: 'Reconciling the device catalog.' },
  synchronized: { label: 'IN SYNC', detail: 'Phone and timer hold the same catalog.' },
  unsynchronized: { label: 'DRAFT', detail: 'Saved on this iPhone, not yet on the timer.' },
  'awaiting-confirmation': {
    label: 'AWAITING TOUCH',
    detail: 'Short-press the timer knob to accept. Hold it to reject.',
  },
  busy: { label: 'TIMER BUSY', detail: 'Finish or cancel the active session, then retry.' },
  conflict: { label: 'CATALOG CHANGED', detail: 'The device changed first. Reconnect and review.' },
  expired: { label: 'RUNE EXPIRED', detail: 'The 15-second confirmation window closed.' },
  rejected: { label: 'DECLINED', detail: 'The physical timer rejected this draft.' },
  'storage-failed': { label: 'NOT SAVED', detail: 'The timer could not commit the catalog.' },
};

function minutes(milliseconds: number): string {
  return String(Math.round(milliseconds / 60_000));
}

export default function PresetsScreen() {
  const { connection, presetCatalog, presetSync, updatePresetDraft, submitPresetDraft } =
    useCompanionRuntime();
  const copy = syncCopy[presetSync];
  const connected = connection.phase === 'ready';
  const canSend = connected && presetCatalog !== null && presetSync !== 'awaiting-confirmation';
  const validation = useMemo(() => validateDraft(presetCatalog?.draft ?? []), [presetCatalog]);

  const replace = (index: number, patch: Partial<PresetDraft>) => {
    if (presetCatalog === null) return;
    const next = presetCatalog.draft.map((preset, current) =>
      current === index ? { ...preset, ...patch } : preset,
    );
    void updatePresetDraft(next);
  };

  const move = (index: number, offset: -1 | 1) => {
    if (presetCatalog === null) return;
    const destination = index + offset;
    if (destination < 0 || destination >= presetCatalog.draft.length) return;
    const next = [...presetCatalog.draft];
    [next[index], next[destination]] = [next[destination]!, next[index]!];
    void updatePresetDraft(next);
  };

  const add = () => {
    if (presetCatalog === null || presetCatalog.draft.length >= 8) return;
    const used = new Set([
      ...presetCatalog.builtIns.map(({ id }) => id),
      ...presetCatalog.draft.map(({ id }) => id),
    ]);
    let suffix = presetCatalog.draft.length + 1;
    while (used.has(`ritual-${suffix}`)) suffix += 1;
    void updatePresetDraft([
      ...presetCatalog.draft,
      { id: `ritual-${suffix}`, name: 'New ritual', plannedDurationMs: 25 * 60_000 },
    ]);
  };

  return (
    <Page>
      <View style={styles.heading}>
        <Text style={styles.eyebrow}>RITUAL CATALOG</Text>
        <Text accessibilityRole="header" style={styles.title}>
          Shape your focus.
        </Text>
        <Text style={styles.detail}>
          Built-ins are carved into the timer. Your rituals remain editable and travel with it.
        </Text>
      </View>

      <View style={styles.syncCard}>
        <Sigil name="norns" size={34} />
        <View style={styles.syncCopy}>
          <Text style={styles.syncLabel}>{copy.label}</Text>
          <Text style={styles.syncDetail}>{copy.detail}</Text>
        </View>
        <Text style={styles.revision}>R{presetCatalog?.revision ?? '—'}</Text>
      </View>

      <Text style={styles.sectionLabel}>CARVED PRESETS</Text>
      <View style={styles.builtIns}>
        {(presetCatalog?.builtIns ?? []).map((preset, index) => (
          <View key={preset.id} style={styles.builtInRow}>
            <Text style={styles.index}>{String(index + 1).padStart(2, '0')}</Text>
            <View style={styles.rowCopy}>
              <Text style={styles.presetName}>{preset.name}</Text>
              <Text style={styles.presetMeta}>{minutes(preset.plannedDurationMs)} min · fixed</Text>
            </View>
            <View style={styles.fixedMark} />
          </View>
        ))}
        {presetCatalog === null ? (
          <Text style={styles.empty}>Connect to read the catalog.</Text>
        ) : null}
      </View>

      <View style={styles.sectionHead}>
        <Text style={styles.sectionLabel}>YOUR RITUALS</Text>
        <Text style={styles.capacity}>{presetCatalog?.draft.length ?? 0} / 8</Text>
      </View>
      <View style={styles.customList}>
        {(presetCatalog?.draft ?? []).map((preset, index) => (
          <View key={preset.id} style={styles.editorCard}>
            <View style={styles.editorRail}>
              <Text style={styles.editorIndex}>{String(index + 1).padStart(2, '0')}</Text>
              <View style={styles.reorder}>
                <Pressable accessibilityLabel="Move preset up" onPress={() => move(index, -1)}>
                  <Text style={styles.reorderText}>↑</Text>
                </Pressable>
                <Pressable accessibilityLabel="Move preset down" onPress={() => move(index, 1)}>
                  <Text style={styles.reorderText}>↓</Text>
                </Pressable>
              </View>
            </View>
            <View style={styles.fields}>
              <TextInput
                accessibilityLabel="Preset name"
                maxLength={32}
                onChangeText={(name) => replace(index, { name })}
                placeholder="Ritual name"
                placeholderTextColor={color.faintText}
                style={styles.nameInput}
                value={preset.name}
              />
              <View style={styles.durationRow}>
                <TextInput
                  accessibilityLabel="Duration in minutes"
                  keyboardType="number-pad"
                  maxLength={3}
                  onChangeText={(value) =>
                    replace(index, { plannedDurationMs: Number(value || 0) * 60_000 })
                  }
                  style={styles.durationInput}
                  value={minutes(preset.plannedDurationMs)}
                />
                <Text style={styles.minuteLabel}>MINUTES</Text>
                <Text style={styles.stableId}>{preset.id}</Text>
              </View>
            </View>
            <Pressable
              accessibilityLabel="Delete preset"
              onPress={() =>
                void updatePresetDraft(
                  presetCatalog!.draft.filter((_, current) => current !== index),
                )
              }
              style={styles.deleteButton}
            >
              <Text style={styles.deleteText}>×</Text>
            </Pressable>
          </View>
        ))}
      </View>

      {validation !== null ? <Text style={styles.validation}>{validation}</Text> : null}
      <View style={styles.actions}>
        <Pressable
          accessibilityRole="button"
          disabled={presetCatalog === null || presetCatalog.draft.length >= 8}
          onPress={add}
          style={styles.secondaryButton}
        >
          <Text style={styles.secondaryText}>＋ Add ritual</Text>
        </Pressable>
        <Pressable
          accessibilityRole="button"
          disabled={!canSend || validation !== null}
          onPress={() => void submitPresetDraft()}
          style={[styles.primaryButton, (!canSend || validation !== null) && styles.disabled]}
        >
          <Text style={styles.primaryText}>
            {presetSync === 'awaiting-confirmation' ? 'Touch the timer' : 'Send to timer'}
          </Text>
        </Pressable>
      </View>
    </Page>
  );
}

function validateDraft(entries: readonly PresetDraft[]): string | null {
  if (entries.length > 8) return 'A timer can hold at most eight custom rituals.';
  const ids = new Set<string>();
  for (const entry of entries) {
    if (entry.name.trim().length === 0) return 'Every ritual needs a name.';
    if (new TextEncoder().encode(entry.name).length > 32) return 'Names must fit within 32 bytes.';
    if (ids.has(entry.id)) return 'Ritual identifiers must stay unique.';
    ids.add(entry.id);
    const durationMinutes = entry.plannedDurationMs / 60_000;
    if (!Number.isInteger(durationMinutes) || durationMinutes < 1 || durationMinutes > 720) {
      return 'Durations must be whole minutes from 1 to 720.';
    }
  }
  return null;
}

const styles = StyleSheet.create({
  heading: { paddingBottom: space.lg, paddingTop: space.lg },
  eyebrow: { color: color.accent, fontSize: 10, fontWeight: '800', letterSpacing: 2 },
  title: { color: color.text, fontSize: 31, fontWeight: '700', letterSpacing: -1, marginTop: 9 },
  detail: { color: color.mutedText, fontSize: 13, lineHeight: 19, marginTop: 8 },
  syncCard: {
    alignItems: 'center',
    backgroundColor: color.surface,
    borderColor: color.lineBright,
    borderRadius: radius.panel,
    borderWidth: 1,
    flexDirection: 'row',
    gap: 12,
    padding: 15,
  },
  syncCopy: { flex: 1 },
  syncLabel: { color: color.accent, fontSize: 10, fontWeight: '800', letterSpacing: 1.5 },
  syncDetail: { color: color.mutedText, fontSize: 12, lineHeight: 17, marginTop: 4 },
  revision: { color: color.faintText, fontSize: 10, fontWeight: '800' },
  sectionHead: {
    alignItems: 'center',
    flexDirection: 'row',
    justifyContent: 'space-between',
    marginTop: space.xl,
  },
  sectionLabel: {
    color: color.faintText,
    fontSize: 9,
    fontWeight: '800',
    letterSpacing: 1.8,
    marginBottom: 9,
    marginTop: space.xl,
  },
  capacity: { color: color.accent, fontSize: 10, fontWeight: '800', marginTop: space.md },
  builtIns: {
    borderColor: color.line,
    borderRadius: radius.panel,
    borderWidth: 1,
    overflow: 'hidden',
  },
  builtInRow: {
    alignItems: 'center',
    borderBottomColor: color.line,
    borderBottomWidth: StyleSheet.hairlineWidth,
    flexDirection: 'row',
    minHeight: 58,
    paddingHorizontal: 14,
  },
  index: { color: color.faintText, fontSize: 10, fontWeight: '800', width: 30 },
  rowCopy: { flex: 1 },
  presetName: { color: color.text, fontSize: 14, fontWeight: '700' },
  presetMeta: { color: color.mutedText, fontSize: 11, marginTop: 3 },
  fixedMark: {
    borderColor: color.accent,
    borderWidth: 1,
    height: 9,
    transform: [{ rotate: '45deg' }],
    width: 9,
  },
  empty: { color: color.mutedText, padding: 18 },
  customList: { gap: 10 },
  editorCard: {
    alignItems: 'stretch',
    backgroundColor: color.backgroundLifted,
    borderColor: color.line,
    borderRadius: radius.panel,
    borderWidth: 1,
    flexDirection: 'row',
    overflow: 'hidden',
  },
  editorRail: {
    alignItems: 'center',
    backgroundColor: color.surface,
    borderRightColor: color.line,
    borderRightWidth: 1,
    justifyContent: 'space-between',
    padding: 10,
    width: 48,
  },
  editorIndex: { color: color.accent, fontSize: 10, fontWeight: '800' },
  reorder: { flexDirection: 'row', gap: 7 },
  reorderText: { color: color.mutedText, fontSize: 15 },
  fields: { flex: 1, padding: 13 },
  nameInput: { color: color.text, fontSize: 16, fontWeight: '700', padding: 0 },
  durationRow: { alignItems: 'center', flexDirection: 'row', marginTop: 10 },
  durationInput: {
    backgroundColor: color.surface,
    borderColor: color.line,
    borderRadius: 8,
    borderWidth: 1,
    color: color.accent,
    fontSize: 14,
    fontWeight: '800',
    paddingHorizontal: 9,
    paddingVertical: 6,
    textAlign: 'center',
    width: 60,
  },
  minuteLabel: {
    color: color.faintText,
    fontSize: 8,
    fontWeight: '800',
    letterSpacing: 1,
    marginLeft: 7,
  },
  stableId: { color: color.faintText, flex: 1, fontSize: 9, marginLeft: 10, textAlign: 'right' },
  deleteButton: { alignItems: 'center', justifyContent: 'center', width: 42 },
  deleteText: { color: color.mutedText, fontSize: 25, fontWeight: '300' },
  validation: { color: '#f2a7a7', fontSize: 12, lineHeight: 17, marginTop: 12 },
  actions: { flexDirection: 'row', gap: 10, marginTop: space.lg },
  secondaryButton: {
    alignItems: 'center',
    borderColor: color.lineBright,
    borderRadius: radius.pill,
    borderWidth: 1,
    flex: 1,
    justifyContent: 'center',
    minHeight: 50,
  },
  secondaryText: { color: color.accent, fontSize: 12, fontWeight: '800' },
  primaryButton: {
    alignItems: 'center',
    backgroundColor: color.accent,
    borderRadius: radius.pill,
    flex: 1.2,
    justifyContent: 'center',
    minHeight: 50,
  },
  primaryText: { color: color.background, fontSize: 12, fontWeight: '900' },
  disabled: { opacity: 0.35 },
});
