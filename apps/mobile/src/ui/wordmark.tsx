import { StyleSheet, Text, View } from 'react-native';

import { color, space } from './theme';

export function Wordmark() {
  return (
    <View style={styles.row}>
      <View style={styles.mark} />
      <Text accessibilityRole="header" style={styles.label}>
        FOCUS / TIMER
      </Text>
    </View>
  );
}

const styles = StyleSheet.create({
  row: {
    alignItems: 'center',
    flexDirection: 'row',
    gap: space.sm,
    paddingTop: space.md,
  },
  mark: {
    width: 9,
    height: 9,
    borderRadius: 5,
    backgroundColor: color.signal,
  },
  label: {
    color: color.ink,
    fontFamily: 'Courier',
    fontSize: 12,
    fontWeight: '700',
    letterSpacing: 1.6,
  },
});
