export type DeviceBackend = 'mock' | 'ble';

export type RuntimeConfig = Readonly<{
  deviceBackend: DeviceBackend;
}>;

export function resolveDeviceBackend(value: string | undefined): DeviceBackend {
  if (value === undefined || value === '' || value === 'mock') {
    return 'mock';
  }

  if (value === 'ble') {
    return value;
  }

  throw new Error(
    `Unsupported EXPO_PUBLIC_DEVICE_BACKEND=${JSON.stringify(value)}; expected "mock" or "ble"`,
  );
}

export const runtimeConfig: RuntimeConfig = {
  deviceBackend: resolveDeviceBackend(process.env.EXPO_PUBLIC_DEVICE_BACKEND),
};
