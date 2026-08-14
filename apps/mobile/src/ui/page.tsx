import { type PropsWithChildren } from 'react';
import { ScrollView, StyleSheet, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { color, space } from './theme';

export function Page({ children }: PropsWithChildren) {
  return (
    <SafeAreaView edges={['left', 'right']} style={styles.safeArea}>
      <View pointerEvents="none" style={styles.ambientTop} />
      <View pointerEvents="none" style={styles.ambientSide} />
      <ScrollView
        contentContainerStyle={styles.content}
        contentInsetAdjustmentBehavior="automatic"
        showsVerticalScrollIndicator={false}
      >
        {children}
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  safeArea: {
    flex: 1,
    backgroundColor: color.background,
  },
  content: {
    flexGrow: 1,
    paddingHorizontal: space.lg,
    paddingBottom: space.xxl,
  },
  ambientTop: {
    position: 'absolute',
    right: -90,
    top: -130,
    width: 300,
    height: 300,
    borderRadius: 150,
    backgroundColor: color.accentWash,
    shadowColor: color.accent,
    shadowOpacity: 0.22,
    shadowRadius: 80,
  },
  ambientSide: {
    position: 'absolute',
    left: -150,
    top: 360,
    width: 230,
    height: 230,
    borderRadius: 115,
    backgroundColor: 'rgba(30, 74, 79, 0.12)',
    shadowColor: color.accent,
    shadowOpacity: 0.12,
    shadowRadius: 60,
  },
});
