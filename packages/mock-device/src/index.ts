export const mockDevicePackageName = '@focus-timer/mock-device';

export {
  createMockDevice,
  DeterministicMockDevice,
  malformedMockResponseBytes,
  type MockDeviceStatus,
  type MockHandshakeResult,
  type MockPageRequest,
  type MockPageResult,
  type MockSessionPage,
} from './mock-device';
export {
  createMockTransport,
  MockDeviceTransport,
  type MockRequestResponder,
} from './mock-transport';
export {
  getMockScenario,
  mockScenarioIds,
  mockScenarios,
  type MockJournal,
  type MockScenario,
  type MockScenarioId,
  type MockSessionOutcome,
  type MockSessionRecord,
  type MockTransportBehavior,
} from './scenarios';
