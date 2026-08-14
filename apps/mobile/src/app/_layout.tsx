import { Stack, usePathname, useRouter } from 'expo-router';
import { StatusBar } from 'expo-status-bar';
import { StyleSheet, View } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';

import { RuntimeProvider } from '@/application/runtime-provider';
import { AppHeader } from '@/ui/app-header';
import { color } from '@/ui/theme';

export default function RootLayout() {
  const pathname = usePathname();
  const router = useRouter();
  const isHome = pathname === '/';

  return (
    <SafeAreaProvider>
      <RuntimeProvider>
        <StatusBar style="light" />
        <View style={styles.shell}>
          <AppHeader canGoBack={!isHome} onBack={() => router.back()} />
          <View style={styles.content}>
            <Stack
              screenOptions={{
                animation: 'slide_from_right',
                contentStyle: { backgroundColor: color.background },
                headerShown: false,
              }}
            >
              <Stack.Screen name="index" />
              <Stack.Screen name="device" />
              <Stack.Screen name="history" />
              <Stack.Screen name="presets" />
            </Stack>
          </View>
        </View>
      </RuntimeProvider>
    </SafeAreaProvider>
  );
}

const styles = StyleSheet.create({
  shell: {
    backgroundColor: color.background,
    flex: 1,
  },
  content: {
    flex: 1,
  },
});
