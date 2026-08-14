import { Image } from 'expo-image';
import { StyleSheet } from 'react-native';

const source = {
  bifrost: require('../../assets/sigils/bifrost-dark.svg'),
  norns: require('../../assets/sigils/norns.svg'),
  time: require('../../assets/sigils/time-chase.svg'),
} as const;

type SigilProps = Readonly<{
  name: keyof typeof source;
  size: number;
}>;

export function Sigil({ name, size }: SigilProps) {
  return (
    <Image
      accessibilityElementsHidden
      contentFit="contain"
      source={source[name]}
      style={[styles.image, { height: size, width: size }]}
    />
  );
}

const styles = StyleSheet.create({
  image: {
    flexShrink: 0,
  },
});
