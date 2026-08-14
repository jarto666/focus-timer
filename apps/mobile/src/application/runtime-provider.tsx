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
  runProtocolFaultAcceptance,
  synchronizeForeground,
  type DeviceCandidate,
  type DeviceClient,
  type DeviceConnectionState,
  type DeviceTransport,
  type DeviceTransportAvailability,
  type ReadyDevice,
} from '@focus-timer/device-client';
import {
  Capability,
  CatalogResult,
  MAX_RECORDS_PER_PAGE,
  PROTOCOL_MAJOR,
  PROTOCOL_MINOR,
  SessionOutcome,
  ViewState,
  ProtocolErrorCode,
  encodeRequest,
} from '@focus-timer/device-protocol';
import {
  createProtocolMockTransport,
  getMockScenario,
  type MockScenario,
  type MockScenarioId,
} from '@focus-timer/mock-device';

import { runtimeConfig } from '@/config/runtime';
import { BleDeviceTransport, createBleDeviceTransport } from '@/ble/ble-device-transport';
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
  type PresetDraft,
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

function statusModel(
  status: Awaited<ReturnType<typeof synchronizeForeground>>['status'],
  freshness: DeviceStatusModel['freshness'] = 'live',
) {
  const states: Record<ViewState, DeviceStatusModel['viewState']> = {
    [ViewState.Idle]: 'idle',
    [ViewState.Running]: 'running',
    [ViewState.Paused]: 'paused',
    [ViewState.Completed]: 'completed',
  };
  return {
    presetId: status.preset.id,
    presetName: status.preset.name,
    plannedDurationMs: status.preset.plannedDurationMs,
    remainingDurationMs: status.remainingDurationMs,
    viewState: states[status.viewState],
    clockKnown: status.clockKnown,
    observedAtMs: Date.now(),
    freshness,
  } satisfies DeviceStatusModel;
}

function catalogModel(
  catalog: Awaited<ReturnType<DeviceClient['getPresetCatalog']>>,
  draft: readonly PresetDraft[],
  baseRevision = catalog.revision,
) {
  return {
    revision: catalog.revision,
    baseRevision,
    builtIns: catalog.entries.filter((entry) => entry.builtIn),
    committedCustom: catalog.entries.filter((entry) => !entry.builtIn),
    draft,
  };
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
  const [presetCatalog, setPresetCatalog] = useState<CompanionRuntime['presetCatalog']>(null);
  const [presetSync, setPresetSync] = useState<CompanionRuntime['presetSync']>('unavailable');
  const repository = useRef<SqliteSessionRepository | null>(null);
  const currentDevice = useRef<ReadyDevice | null>(null);
  const candidates = useRef(new Map<string, DeviceCandidate>());
  const automaticReconnectAttempt = useRef<string | null>(null);
  const physicalAcceptanceRun = useRef(false);
  const latestAvailability = useRef<DeviceTransportAvailability>({ status: 'available' });
  const deviceClient = useRef<DeviceClient | null>(null);
  const unsubscribeEvents = useRef<(() => void) | null>(null);
  const recoveryInFlight = useRef(false);

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
      unsubscribeEvents.current?.();
      unsubscribeEvents.current = null;
      deviceClient.current = null;
      setStatus((current) => (current === null ? null : { ...current, freshness: 'stale' }));
      setPresetSync((current) => (current === 'synchronized' ? 'unavailable' : current));
      const availability = latestAvailability.current;
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
      setConnection({
        phase: 'disconnected',
        reason: 'link-loss',
        lastDevice: currentDevice.current,
      });
    });
  }, [backend]);

  useEffect(
    () => () => {
      unsubscribeEvents.current?.();
      unsubscribeEvents.current = null;
      deviceClient.current = null;
    },
    [backend],
  );

  useEffect(() => {
    return backend.transport.subscribeToAvailability((availability) => {
      latestAvailability.current = availability;
      if (availability.status === 'available') {
        setConnection((current) => {
          if (current.phase !== 'unavailable' && current.phase !== 'permission-denied') {
            return current;
          }
          automaticReconnectAttempt.current = null;
          return {
            phase: 'disconnected',
            reason: 'initial',
            lastDevice: currentDevice.current,
          };
        });
        return;
      }

      setStatus((current) => (current === null ? null : { ...current, freshness: 'stale' }));
      if (availability.status === 'unavailable') {
        setConnection({ phase: 'unavailable', reason: availability.reason });
      } else {
        setConnection({
          phase: 'permission-denied',
          canOpenSettings: availability.canOpenSettings,
        });
      }
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
          Date.now,
          operation,
        );
        const device = readyDevice(result.device)!;
        currentDevice.current = device;
        deviceClient.current = result.client;
        unsubscribeEvents.current?.();
        const refreshStatus = () => {
          if (recoveryInFlight.current) return;
          recoveryInFlight.current = true;
          void result.client
            .getStatus(operation)
            .then((latest) => setStatus(statusModel(latest)))
            .catch(() =>
              setStatus((current) =>
                current === null ? null : { ...current, freshness: 'stale' },
              ),
            )
            .finally(() => {
              recoveryInFlight.current = false;
            });
        };
        if (result.client.hello?.capabilities.includes(Capability.LiveStatus) === true) {
          unsubscribeEvents.current = result.client.subscribeToEvents((event) => {
            if (event.type === 'liveStatus') {
              setStatus(statusModel(event.status));
              return;
            }
            const catalogResult = event.result;
            switch (catalogResult.result) {
              case CatalogResult.Committed:
                setPresetSync('loading');
                void result.client
                  .getPresetCatalog(operation)
                  .then(async (catalog) => {
                    await repository.current?.clearPresetDraft(result.deviceId);
                    setPresetCatalog(
                      catalogModel(
                        catalog,
                        catalog.entries.filter((entry) => !entry.builtIn),
                      ),
                    );
                    setPresetSync('synchronized');
                  })
                  .catch(() => setPresetSync('unavailable'));
                break;
              case CatalogResult.Rejected:
                setPresetSync('rejected');
                break;
              case CatalogResult.Expired:
                setPresetSync('expired');
                break;
              case CatalogResult.Cancelled:
                setPresetSync('unsynchronized');
                break;
              case CatalogResult.StorageFailed:
                setPresetSync('storage-failed');
                break;
            }
          }, refreshStatus);
          // Subscription is active before this recovery read, closing the startup race.
          setStatus(statusModel(await result.client.getStatus(operation)));
        } else {
          unsubscribeEvents.current = null;
          setStatus(statusModel(result.status));
        }
        if (result.client.hello?.capabilities.includes(Capability.ReadPresetCatalog) === true) {
          setPresetSync('loading');
          const catalog = await result.client.getPresetCatalog(operation);
          const storedDraft = await repository.current.loadPresetDraft(result.deviceId);
          const committedCustom = catalog.entries.filter((entry) => !entry.builtIn);
          const draft = storedDraft?.customEntries ?? committedCustom;
          setPresetCatalog(catalogModel(catalog, draft, storedDraft?.baseRevision));
          setPresetSync(
            JSON.stringify(draft) === JSON.stringify(committedCustom)
              ? 'synchronized'
              : 'unsynchronized',
          );
        } else {
          setPresetCatalog(null);
          setPresetSync('unavailable');
        }
        setConnection({ phase: 'ready', device });
        setHistory(await loadHistory(repository.current, result.deviceId));
        setHistorySync({ phase: 'ready' });
        if (
          runtimeConfig.bleAcceptanceDiagnostic === 'fault-matrix' &&
          backend.transport instanceof BleDeviceTransport &&
          !physicalAcceptanceRun.current
        ) {
          physicalAcceptanceRun.current = true;
          console.info('[BLE acceptance] starting physical read-only fault matrix');
          const corruptFrame = await backend.transport.writeCorruptFrameForAcceptance(operation);
          const protocolFaults = await runProtocolFaultAcceptance(backend.transport, operation);
          const pageRequest = encodeRequest({
            version: { major: PROTOCOL_MAJOR, minor: PROTOCOL_MINOR },
            requestId: 0xfa00_0100,
            request: {
              type: 'getSessionPage',
              page: { afterSequence: 0, limit: MAX_RECORDS_PER_PAGE },
            },
          });
          await backend.transport.disconnectDuringResponseForAcceptance(pageRequest, operation);
          console.info(
            `[BLE acceptance] complete ${JSON.stringify({ corruptFrame, ...protocolFaults, disconnectedDuringResponse: true })}`,
          );
          setConnection({
            phase: 'disconnected',
            reason: 'link-loss',
            lastDevice: currentDevice.current,
          });
          setStatus((current) => (current === null ? null : { ...current, freshness: 'stale' }));
        }
      } catch (error) {
        unsubscribeEvents.current?.();
        unsubscribeEvents.current = null;
        deviceClient.current = null;
        setStatus((current) => (current === null ? null : { ...current, freshness: 'stale' }));
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

  useEffect(() => {
    if (
      runtimeConfig.deviceBackend !== 'ble' ||
      historySync.phase !== 'ready' ||
      connection.phase !== 'disconnected' ||
      connection.reason !== 'initial' ||
      connection.lastDevice === null
    ) {
      return;
    }

    const reconnectKey = `${connection.lastDevice.deviceId}:${connection.lastDevice.transportId}`;
    if (automaticReconnectAttempt.current === reconnectKey) return;
    automaticReconnectAttempt.current = reconnectKey;
    void connect(connection.lastDevice.transportId);
  }, [connect, connection, historySync.phase]);

  const disconnect = useCallback(async () => {
    try {
      unsubscribeEvents.current?.();
      unsubscribeEvents.current = null;
      deviceClient.current = null;
      await backend.transport.disconnect();
      setConnection({
        phase: 'disconnected',
        reason: 'user',
        lastDevice: currentDevice.current,
      });
      setStatus((current) => (current === null ? null : { ...current, freshness: 'stale' }));
      setPresetSync((current) => (current === 'synchronized' ? 'unavailable' : current));
    } catch (error) {
      setConnection(retryableState(error, 'disconnect', null));
    }
  }, [backend]);

  const selectScenario = useCallback((scenarioId: MockScenarioId) => {
    if (runtimeConfig.deviceBackend !== 'mock') return;
    const nextBackend = createBackend(scenarioId);
    setSelectedScenario(scenarioId);
    setBackend(nextBackend);
    unsubscribeEvents.current?.();
    unsubscribeEvents.current = null;
    deviceClient.current = null;
    candidates.current.clear();
    setConnection(initialConnection());
    setStatus(null);
    setPresetCatalog(null);
    setPresetSync('unavailable');
  }, []);

  const updatePresetDraft = useCallback(
    async (entries: readonly PresetDraft[]) => {
      const device = currentDevice.current;
      const store = repository.current;
      if (device === null || store === null || presetCatalog === null) return;
      await store.savePresetDraft(device.deviceId, presetCatalog.baseRevision, entries);
      setPresetCatalog({ ...presetCatalog, draft: entries });
      setPresetSync(
        JSON.stringify(entries) === JSON.stringify(presetCatalog.committedCustom)
          ? 'synchronized'
          : 'unsynchronized',
      );
    },
    [presetCatalog],
  );

  const submitPresetDraft = useCallback(async () => {
    const client = deviceClient.current;
    const device = currentDevice.current;
    const store = repository.current;
    if (client === null || device === null || store === null || presetCatalog === null) {
      setPresetSync('unavailable');
      return;
    }
    setPresetSync('awaiting-confirmation');
    try {
      const proposalId = (Date.now() % 0xffff_fffe) + 1;
      await client.proposePresetCatalog(
        {
          expectedRevision: presetCatalog.baseRevision,
          proposalId,
          customEntries: presetCatalog.draft,
        },
        operation,
      );
      if (backend.transport.kind === 'mock') {
        const catalog = await client.getPresetCatalog(operation);
        await store.clearPresetDraft(device.deviceId);
        const committed = catalog.entries.filter((entry) => !entry.builtIn);
        setPresetCatalog(catalogModel(catalog, committed));
        setPresetSync('synchronized');
      }
    } catch (error) {
      if (error instanceof DeviceClientError && error.code === 'remote-error') {
        if (error.details.protocolErrorCode === ProtocolErrorCode.Busy) {
          setPresetSync('busy');
          return;
        }
        if (error.details.protocolErrorCode === ProtocolErrorCode.CatalogConflict) {
          setPresetSync('conflict');
          return;
        }
      }
      setPresetSync('unavailable');
    }
  }, [backend.transport.kind, presetCatalog]);

  const value = useMemo<CompanionRuntime>(
    () => ({
      connection,
      status,
      history,
      historySync,
      presetCatalog,
      presetSync,
      selectedScenario,
      developmentScenarios: runtimeConfig.deviceBackend === 'mock' ? developmentScenarios : [],
      startScan,
      connect,
      disconnect,
      updatePresetDraft,
      submitPresetDraft,
      selectScenario,
    }),
    [
      connection,
      connect,
      disconnect,
      history,
      historySync,
      presetCatalog,
      presetSync,
      selectScenario,
      selectedScenario,
      startScan,
      status,
      submitPresetDraft,
      updatePresetDraft,
    ],
  );

  return <RuntimeContext.Provider value={value}>{children}</RuntimeContext.Provider>;
}

export function useCompanionRuntime(): CompanionRuntime {
  const value = useContext(RuntimeContext);
  if (value === null) throw new Error('useCompanionRuntime must be used inside RuntimeProvider');
  return value;
}
