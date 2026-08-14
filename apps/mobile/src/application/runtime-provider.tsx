import {
  createContext,
  type PropsWithChildren,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

import {
  DeviceTransportError,
  type DeviceCandidate,
  type DeviceConnectionState,
  type ReadyDevice,
} from '@focus-timer/device-client';
import {
  createMockDevice,
  createMockTransport,
  getMockScenario,
  type DeterministicMockDevice,
  type MockDeviceTransport,
  type MockScenario,
  type MockScenarioId,
} from '@focus-timer/mock-device';

import { runtimeConfig } from '@/config/runtime';

import {
  type CompanionRuntime,
  type DevelopmentScenarioOption,
  emptyHistory,
} from './companion-model';
import { synchronizeMockHistory } from './mock-sync';

type MockBackend = Readonly<{
  scenario: MockScenario;
  device: DeterministicMockDevice;
  transport: MockDeviceTransport;
}>;

const operation = { timeoutMs: 1_000 } as const;

const scenarioLabels: Record<MockScenarioId, string> = {
  empty: 'Empty',
  paged: 'Paged',
  'duplicated-page': 'Duplicate',
  gap: 'Gap',
  'changed-epoch': 'New epoch',
  disconnect: 'Disconnect',
  'incompatible-version': 'Incompatible',
  'malformed-response': 'Malformed',
  'unavailable-device': 'Unavailable',
};

const developmentScenarios: readonly DevelopmentScenarioOption[] = Object.entries(
  scenarioLabels,
).map(([id, label]) => ({ id: id as MockScenarioId, label }));

const RuntimeContext = createContext<CompanionRuntime | null>(null);

function createBackend(scenarioId: MockScenarioId): MockBackend | null {
  if (runtimeConfig.deviceBackend !== 'mock') {
    return null;
  }

  const scenario = getMockScenario(scenarioId);
  return {
    scenario,
    device: createMockDevice(scenario),
    transport: createMockTransport(scenario),
  };
}

function initialConnection(backend: MockBackend | null): DeviceConnectionState {
  return backend === null
    ? { phase: 'unavailable', reason: 'unsupported' }
    : { phase: 'disconnected', reason: 'initial', lastDevice: null };
}

function retryableState(
  error: unknown,
  operationName: 'scan' | 'connect' | 'handshake' | 'request' | 'disconnect',
  candidate: DeviceCandidate | null,
): DeviceConnectionState {
  const message = error instanceof Error ? error.message : 'Unknown device error';
  let code: Extract<DeviceConnectionState, { phase: 'retryable-error' }>['error']['code'] =
    'transport-failed';

  if (error instanceof DeviceTransportError) {
    if (error.code === 'connection-lost') {
      code = 'connection-lost';
    } else if (error.code === 'request-timeout') {
      code = 'request-timeout';
    } else if (operationName === 'scan') {
      code = 'scan-failed';
    } else if (operationName === 'connect') {
      code = 'connect-failed';
    }
  }

  return {
    phase: 'retryable-error',
    candidate,
    error: { code, operation: operationName, message },
  };
}

export function RuntimeProvider({ children }: PropsWithChildren) {
  const [selectedScenario, setSelectedScenario] = useState(runtimeConfig.mockScenario);
  const [backend, setBackend] = useState<MockBackend | null>(() =>
    createBackend(runtimeConfig.mockScenario),
  );
  const [connection, setConnection] = useState<DeviceConnectionState>(() =>
    initialConnection(backend),
  );
  const [status, setStatus] = useState<CompanionRuntime['status']>(null);
  const [history, setHistory] = useState(emptyHistory);
  const readyDevice = useRef<ReadyDevice | null>(null);

  useEffect(() => {
    if (backend === null) {
      return;
    }

    return backend.transport.subscribeToDisconnect(() => {
      setConnection({
        phase: 'disconnected',
        reason: 'link-loss',
        lastDevice: readyDevice.current,
      });
      setStatus(null);
    });
  }, [backend]);

  const startScan = useCallback(async () => {
    if (backend === null) {
      setConnection({ phase: 'unavailable', reason: 'unsupported' });
      return;
    }

    const availability = await backend.transport.readAvailability();
    if (availability.status === 'unavailable') {
      setConnection({ phase: 'unavailable', reason: availability.reason });
      return;
    }
    if (availability.status === 'permission-denied') {
      setConnection({
        phase: 'permission-denied',
        canOpenSettings: availability.canOpenSettings,
      });
      return;
    }

    setConnection({ phase: 'scanning', candidates: [] });
    try {
      const candidates = await backend.transport.scan(operation);
      setConnection({ phase: 'scanning', candidates });
    } catch (error) {
      setConnection(retryableState(error, 'scan', null));
    }
  }, [backend]);

  const connect = useCallback(
    async (transportId: string) => {
      if (backend === null) {
        setConnection({ phase: 'unavailable', reason: 'unsupported' });
        return;
      }

      const candidate = backend.scenario.candidate;
      if (candidate.transportId !== transportId) {
        setConnection(
          retryableState(new Error('Selected timer is no longer available'), 'connect', null),
        );
        return;
      }

      setConnection({ phase: 'connecting', candidate });
      try {
        await backend.transport.connect(candidate, operation);
        setConnection({ phase: 'handshaking', candidate });

        // Exercises transport failures without defining a temporary wire protocol.
        await backend.transport.request(Uint8Array.of(0), operation);
        const handshake = backend.device.handshake();
        if (handshake.kind === 'incompatible') {
          await backend.transport.disconnect();
          setConnection({
            phase: 'incompatible',
            candidate,
            supportedMajor: handshake.supportedMajor,
            receivedMajor: handshake.receivedMajor,
          });
          return;
        }

        readyDevice.current = handshake.device;
        setConnection({ phase: 'ready', device: handshake.device });
        const deviceStatus = backend.device.readStatus();
        setStatus({
          presetName: deviceStatus.selectedPreset.name,
          plannedDurationMs: deviceStatus.selectedPreset.durationMs,
          remainingDurationMs: deviceStatus.remainingDurationMs,
          viewState: deviceStatus.viewState,
          clockKnown: deviceStatus.clockKnown,
        });
        setHistory(synchronizeMockHistory(backend.device, backend.scenario));
      } catch (error) {
        setConnection(retryableState(error, 'handshake', candidate));
      }
    },
    [backend],
  );

  const disconnect = useCallback(async () => {
    if (backend === null) {
      return;
    }

    const lastDevice = readyDevice.current;
    try {
      await backend.transport.disconnect();
      readyDevice.current = null;
      setConnection({ phase: 'disconnected', reason: 'user', lastDevice });
      setStatus(null);
    } catch (error) {
      setConnection(retryableState(error, 'disconnect', null));
    }
  }, [backend]);

  const selectScenario = useCallback((scenarioId: MockScenarioId) => {
    const nextBackend = createBackend(scenarioId);
    readyDevice.current = null;
    setSelectedScenario(scenarioId);
    setBackend(nextBackend);
    setConnection(initialConnection(nextBackend));
    setStatus(null);
    setHistory(emptyHistory);
  }, []);

  const value = useMemo<CompanionRuntime>(
    () => ({
      connection,
      status,
      history,
      selectedScenario,
      developmentScenarios: runtimeConfig.deviceBackend === 'mock' ? developmentScenarios : [],
      startScan,
      connect,
      disconnect,
      selectScenario,
    }),
    [connection, connect, disconnect, history, selectScenario, selectedScenario, startScan, status],
  );

  return <RuntimeContext.Provider value={value}>{children}</RuntimeContext.Provider>;
}

export function useCompanionRuntime(): CompanionRuntime {
  const value = useContext(RuntimeContext);

  if (value === null) {
    throw new Error('useCompanionRuntime must be used inside RuntimeProvider');
  }

  return value;
}
