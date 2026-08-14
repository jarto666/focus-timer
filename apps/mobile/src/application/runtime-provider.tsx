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
  DeviceClientError,
  DeviceTransportError,
  synchronizeForeground,
  type DeviceCandidate,
  type DeviceConnectionState,
  type DeviceTransport,
  type ReadyDevice,
} from '@focus-timer/device-client';
import { SessionOutcome, ViewState } from '@focus-timer/device-protocol';
import {
  createProtocolMockTransport,
  getMockScenario,
  type MockScenario,
  type MockScenarioId,
} from '@focus-timer/mock-device';

import { runtimeConfig } from '@/config/runtime';
import { createBleDeviceTransport } from '@/ble/ble-device-transport';
import {
  openSessionRepository,
  type SqliteSessionRepository,
} from '@/storage/sqlite-session-repository';

import {
  type CompanionRuntime,
  type DevelopmentScenarioOption,
  type DeviceStatusModel,
  type LocalHistoryModel,
  emptyHistory,
} from './companion-model';

type AppBackend = Readonly<{
  scenario: MockScenario | null;
  transport: DeviceTransport;
}>;

const operation = { timeoutMs: 5_000 } as const;

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

function createBackend(scenarioId: MockScenarioId): AppBackend {
  if (runtimeConfig.deviceBackend === 'ble') {
    return { scenario: null, transport: createBleDeviceTransport() };
  }
  const scenario = getMockScenario(scenarioId);
  return { scenario, transport: createProtocolMockTransport(scenario) };
}

function initialConnection(): DeviceConnectionState {
  return { phase: 'disconnected', reason: 'initial', lastDevice: null };
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
    if (error.code === 'connection-lost') code = 'connection-lost';
    else if (error.code === 'request-timeout') code = 'request-timeout';
    else if (operationName === 'scan') code = 'scan-failed';
    else if (operationName === 'connect') code = 'connect-failed';
  }

  return {
    phase: 'retryable-error',
    candidate,
    error: { code, operation: operationName, message },
  };
}

function readyDevice(device: Awaited<ReturnType<SqliteSessionRepository['loadMostRecentDevice']>>) {
  if (device === null) return null;
  return {
    deviceId: device.deviceId,
    transportId: device.transportId,
    productName: device.productName,
    firmwareVersion: device.firmwareVersion,
    protocolVersion: device.protocolVersion,
  } satisfies ReadyDevice;
}

function statusModel(status: Awaited<ReturnType<typeof synchronizeForeground>>['status']) {
  const states: Record<ViewState, DeviceStatusModel['viewState']> = {
    [ViewState.Idle]: 'idle',
    [ViewState.Running]: 'running',
    [ViewState.Paused]: 'paused',
    [ViewState.Completed]: 'completed',
  };
  return {
    presetName: status.preset.name,
    plannedDurationMs: status.preset.plannedDurationMs,
    remainingDurationMs: status.remainingDurationMs,
    viewState: states[status.viewState],
    clockKnown: status.clockKnown,
  } satisfies DeviceStatusModel;
}

async function loadHistory(
  repository: SqliteSessionRepository,
  deviceId: string,
): Promise<LocalHistoryModel> {
  const [cursor, sessions] = await Promise.all([
    repository.loadActiveCursor(deviceId),
    repository.listSessions(deviceId),
  ]);
  return {
    entries: sessions.map(({ journalEpoch, record }) => ({
      key: `${journalEpoch}:${record.sequence}`,
      sequence: record.sequence,
      presetName: record.preset.name,
      plannedDurationMs: record.preset.plannedDurationMs,
      activeDurationMs: record.activeDurationMs,
      outcome: record.outcome === SessionOutcome.Completed ? 'completed' : 'cancelled',
      startedAtUtcMs: record.startedAtUtcMs ?? null,
      endedAtUtcMs: record.endedAtUtcMs ?? null,
    })),
    completeness: cursor?.completeness ?? 'complete',
    journalEpoch: cursor?.journalEpoch ?? null,
  };
}

export function RuntimeProvider({ children }: PropsWithChildren) {
  const [selectedScenario, setSelectedScenario] = useState(runtimeConfig.mockScenario);
  const [backend, setBackend] = useState<AppBackend>(() =>
    createBackend(runtimeConfig.mockScenario),
  );
  const [connection, setConnection] = useState<DeviceConnectionState>(() => initialConnection());
  const [status, setStatus] = useState<CompanionRuntime['status']>(null);
  const [history, setHistory] = useState(emptyHistory);
  const [historySync, setHistorySync] = useState<CompanionRuntime['historySync']>({
    phase: 'loading',
  });
  const repository = useRef<SqliteSessionRepository | null>(null);
  const currentDevice = useRef<ReadyDevice | null>(null);
  const candidates = useRef(new Map<string, DeviceCandidate>());

  useEffect(() => {
    let active = true;
    void openSessionRepository()
      .then(async (opened) => {
        repository.current = opened;
        const remembered = await opened.loadMostRecentDevice();
        if (!active) return;
        const lastDevice = readyDevice(remembered);
        currentDevice.current = lastDevice;
        if (lastDevice !== null) {
          setHistory(await loadHistory(opened, lastDevice.deviceId));
          setConnection({ phase: 'disconnected', reason: 'initial', lastDevice });
        }
        setHistorySync({ phase: 'ready' });
      })
      .catch((error: unknown) => {
        if (!active) return;
        setHistorySync({
          phase: 'failed',
          message: error instanceof Error ? error.message : 'Unable to open local history',
        });
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    return backend.transport.subscribeToDisconnect(() => {
      setConnection({
        phase: 'disconnected',
        reason: 'link-loss',
        lastDevice: currentDevice.current,
      });
      setStatus(null);
    });
  }, [backend]);

  const startScan = useCallback(async () => {
    try {
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
      const discovered = await backend.transport.scan(operation);
      candidates.current = new Map(
        discovered.map((candidate) => [candidate.transportId, candidate]),
      );
      if (discovered.length === 0) {
        setConnection(retryableState(new Error('No nearby Focus Timer was found'), 'scan', null));
        return;
      }
      setConnection({ phase: 'scanning', candidates: discovered });
    } catch (error) {
      setConnection(retryableState(error, 'scan', null));
    }
  }, [backend]);

  const connect = useCallback(
    async (transportId: string) => {
      const remembered = currentDevice.current;
      const candidate =
        backend.scenario?.candidate.transportId === transportId
          ? backend.scenario.candidate
          : (candidates.current.get(transportId) ??
            (remembered?.transportId === transportId
              ? {
                  transportId,
                  productName: remembered.productName,
                  rssi: null,
                }
              : null));
      if (candidate === null) {
        setConnection(
          retryableState(new Error('Selected timer is no longer available'), 'connect', null),
        );
        return;
      }
      if (repository.current === null) {
        setHistorySync({ phase: 'failed', message: 'Local history is still unavailable' });
        return;
      }

      setConnection({ phase: 'connecting', candidate });
      setHistorySync({ phase: 'syncing' });
      try {
        setConnection({ phase: 'handshaking', candidate });
        const result = await synchronizeForeground(
          backend.transport,
          candidate,
          repository.current,
          Date.now(),
          operation,
        );
        const device = readyDevice(result.device)!;
        currentDevice.current = device;
        setConnection({ phase: 'ready', device });
        setStatus(statusModel(result.status));
        setHistory(await loadHistory(repository.current, result.deviceId));
        setHistorySync({ phase: 'ready' });
      } catch (error) {
        if (error instanceof DeviceClientError && error.code === 'incompatible') {
          await backend.transport.disconnect();
          setConnection({
            phase: 'incompatible',
            candidate,
            supportedMajor: error.details.supportedMajor ?? 1,
            receivedMajor: error.details.receivedMajor ?? 1,
          });
        } else {
          await backend.transport.disconnect().catch(() => undefined);
          setConnection(retryableState(error, 'handshake', candidate));
        }
        setHistorySync({
          phase: 'failed',
          message: error instanceof Error ? error.message : 'Synchronization failed',
        });
      }
    },
    [backend],
  );

  const disconnect = useCallback(async () => {
    try {
      await backend.transport.disconnect();
      setConnection({
        phase: 'disconnected',
        reason: 'user',
        lastDevice: currentDevice.current,
      });
      setStatus(null);
    } catch (error) {
      setConnection(retryableState(error, 'disconnect', null));
    }
  }, [backend]);

  const selectScenario = useCallback((scenarioId: MockScenarioId) => {
    if (runtimeConfig.deviceBackend !== 'mock') return;
    const nextBackend = createBackend(scenarioId);
    setSelectedScenario(scenarioId);
    setBackend(nextBackend);
    candidates.current.clear();
    setConnection(initialConnection());
    setStatus(null);
  }, []);

  const value = useMemo<CompanionRuntime>(
    () => ({
      connection,
      status,
      history,
      historySync,
      selectedScenario,
      developmentScenarios: runtimeConfig.deviceBackend === 'mock' ? developmentScenarios : [],
      startScan,
      connect,
      disconnect,
      selectScenario,
    }),
    [
      connection,
      connect,
      disconnect,
      history,
      historySync,
      selectScenario,
      selectedScenario,
      startScan,
      status,
    ],
  );

  return <RuntimeContext.Provider value={value}>{children}</RuntimeContext.Provider>;
}

export function useCompanionRuntime(): CompanionRuntime {
  const value = useContext(RuntimeContext);
  if (value === null) throw new Error('useCompanionRuntime must be used inside RuntimeProvider');
  return value;
}
