import { mockScenarioIds, type MockScenarioId } from '@focus-timer/mock-device';

export type DeviceBackend = 'mock' | 'ble';
export type BleAcceptanceDiagnostic = 'off' | 'fault-matrix';

export type RuntimeConfig = Readonly<{
  deviceBackend: DeviceBackend;
  mockScenario: MockScenarioId;
  bleAcceptanceDiagnostic: BleAcceptanceDiagnostic;
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

export function resolveMockScenario(value: string | undefined): MockScenarioId {
  if (value === undefined || value === '') {
    return 'paged';
  }

  const scenario = mockScenarioIds.find((candidate) => candidate === value);
  if (scenario !== undefined) {
    return scenario;
  }

  throw new Error(
    `Unsupported EXPO_PUBLIC_MOCK_SCENARIO=${JSON.stringify(value)}; expected one of ${mockScenarioIds.join(', ')}`,
  );
}

export function resolveBleAcceptanceDiagnostic(value: string | undefined): BleAcceptanceDiagnostic {
  if (value === undefined || value === '' || value === 'off') return 'off';
  if (value === 'fault-matrix') return value;
  throw new Error(
    `Unsupported EXPO_PUBLIC_BLE_ACCEPTANCE=${JSON.stringify(value)}; expected "off" or "fault-matrix"`,
  );
}

export const runtimeConfig: RuntimeConfig = {
  deviceBackend: resolveDeviceBackend(process.env.EXPO_PUBLIC_DEVICE_BACKEND),
  mockScenario: resolveMockScenario(process.env.EXPO_PUBLIC_MOCK_SCENARIO),
  bleAcceptanceDiagnostic: resolveBleAcceptanceDiagnostic(process.env.EXPO_PUBLIC_BLE_ACCEPTANCE),
};
