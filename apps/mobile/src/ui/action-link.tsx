import { Link, type Href } from 'expo-router';
import { Pressable, StyleSheet, Text } from 'react-native';

import { color, radius, space } from './theme';

type ActionLinkProps = Readonly<{
  href: Href;
  label: string;
  tone?: 'dark' | 'light';
}>;

export function ActionLink({ href, label, tone = 'dark' }: ActionLinkProps) {
  return (
    <Link asChild href={href}>
      <Pressable
        accessibilityRole="button"
        style={({ pressed }) => [
          styles.base,
          tone === 'dark' ? styles.dark : styles.light,
          pressed && styles.pressed,
        ]}
      >
        <Text style={[styles.label, tone === 'dark' ? styles.darkLabel : styles.lightLabel]}>
          {label}
        </Text>
        <Text
          accessibilityElementsHidden
          importantForAccessibility="no"
          style={[styles.arrow, tone === 'dark' ? styles.darkLabel : styles.lightLabel]}
        >
          →
        </Text>
      </Pressable>
    </Link>
  );
}

const styles = StyleSheet.create({
  base: {
    minHeight: 58,
    alignItems: 'center',
    borderRadius: radius.control,
    flexDirection: 'row',
    justifyContent: 'space-between',
    paddingHorizontal: space.md,
  },
  dark: {
    backgroundColor: color.ink,
  },
  light: {
    backgroundColor: color.raised,
    borderColor: color.line,
    borderWidth: 1,
  },
  pressed: {
    opacity: 0.72,
    transform: [{ scale: 0.99 }],
  },
  label: {
    fontSize: 16,
    fontWeight: '700',
  },
  arrow: {
    fontSize: 22,
  },
  darkLabel: {
    color: color.raised,
  },
  lightLabel: {
    color: color.ink,
  },
});
