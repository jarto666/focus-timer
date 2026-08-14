import { SymbolView } from 'expo-symbols';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { color } from './theme';

type AppHeaderProps = Readonly<{
  canGoBack: boolean;
  onBack: () => void;
}>;

export function AppHeader({ canGoBack, onBack }: AppHeaderProps) {
  return (
    <SafeAreaView edges={['top', 'left', 'right']} style={styles.safeArea}>
      <View style={styles.bar}>
        <View style={styles.side}>
          {canGoBack ? (
            <Pressable accessibilityLabel="Go back" accessibilityRole="button" onPress={onBack}>
              {({ pressed }) => (
                <View style={[styles.backButton, pressed && styles.backButtonPressed]}>
                  <SymbolView
                    fallback={<Text style={styles.backFallback}>‹</Text>}
                    name="chevron.left"
                    size={16}
                    tintColor={color.accent}
                    weight="semibold"
                  />
                </View>
              )}
            </Pressable>
          ) : null}
        </View>

        <View accessibilityRole="header" style={styles.brand}>
          <Text style={styles.name}>MUNINN</Text>
          <View style={styles.brandCut} />
        </View>

        <View style={[styles.side, styles.sideRight]}>
          <View style={styles.localState}>
            <View style={styles.localDot} />
            <Text style={styles.localLabel}>LOCAL</Text>
          </View>
        </View>
      </View>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  safeArea: {
    backgroundColor: color.background,
    borderBottomColor: color.line,
    borderBottomWidth: StyleSheet.hairlineWidth,
  },
  bar: {
    alignItems: 'center',
    flexDirection: 'row',
    height: 58,
    paddingHorizontal: 14,
  },
  side: {
    alignItems: 'flex-start',
    width: 74,
  },
  sideRight: {
    alignItems: 'flex-end',
  },
  backButton: {
    alignItems: 'center',
    backgroundColor: color.surface,
    borderColor: color.line,
    borderRadius: 17,
    borderWidth: 1,
    height: 34,
    justifyContent: 'center',
    width: 34,
  },
  backButtonPressed: {
    backgroundColor: color.surfacePressed,
    transform: [{ scale: 0.96 }],
  },
  backFallback: {
    color: color.accent,
    fontSize: 25,
    lineHeight: 25,
  },
  brand: {
    alignItems: 'center',
    flex: 1,
    justifyContent: 'center',
  },
  name: {
    color: color.text,
    fontSize: 13,
    fontWeight: '800',
    letterSpacing: 3.8,
    marginLeft: 4,
  },
  brandCut: {
    backgroundColor: color.accent,
    height: 1,
    marginTop: 6,
    shadowColor: color.accent,
    shadowOpacity: 0.8,
    shadowRadius: 4,
    width: 18,
  },
  localState: {
    alignItems: 'center',
    flexDirection: 'row',
    gap: 5,
    paddingVertical: 7,
  },
  localDot: {
    backgroundColor: color.accent,
    borderRadius: 3,
    height: 5,
    shadowColor: color.accent,
    shadowOpacity: 0.8,
    shadowRadius: 5,
    width: 5,
  },
  localLabel: {
    color: color.mutedText,
    fontSize: 8,
    fontWeight: '800',
    letterSpacing: 1.1,
  },
});
