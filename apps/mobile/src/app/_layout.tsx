import { Stack } from 'expo-router';
import { StatusBar } from 'expo-status-bar';
import { SafeAreaProvider } from 'react-native-safe-area-context';

import { RuntimeProvider } from '@/application/runtime-provider';
import { color } from '@/ui/theme';

export default function RootLayout() {
  return (
    <SafeAreaProvider>
      <RuntimeProvider>
        <StatusBar style="dark" />
        <Stack
          screenOptions={{
            animation: 'slide_from_right',
            contentStyle: { backgroundColor: color.paper },
            headerBackButtonDisplayMode: 'minimal',
            headerShadowVisible: false,
            headerStyle: { backgroundColor: color.paper },
            headerTintColor: color.ink,
            headerTitleStyle: { fontWeight: '700' },
          }}
        >
          <Stack.Screen name="index" options={{ headerShown: false }} />
          <Stack.Screen name="device" options={{ title: 'Your timer' }} />
          <Stack.Screen name="history" options={{ title: 'History' }} />
        </Stack>
      </RuntimeProvider>
    </SafeAreaProvider>
  );
}
