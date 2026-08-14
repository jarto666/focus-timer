import {
  Capability,
  JournalHealth,
  ProtocolErrorCode,
  SessionOutcome,
  ViewState,
  decodeRequest,
  encodeResponse,
  protocolVersion,
  type RequestEnvelope,
  type Response,
  type SessionRecord,
} from '@focus-timer/device-protocol';

import { DeterministicMockDevice } from './mock-device';
import { createMockTransport, type MockRequestResponder } from './mock-transport';
import type { MockScenario, MockScenarioId, MockSessionRecord } from './scenarios';

export function createProtocolMockTransport(scenario: MockScenario | MockScenarioId) {
  const device = new DeterministicMockDevice(scenario);
  return createMockTransport(scenario, createProtocolResponder(device));
}

export function createProtocolResponder(device: DeterministicMockDevice): MockRequestResponder {
  return (payload) => {
    const request = decodeRequest(payload);
    const response = respond(device, request);
    return encodeResponse({
      version:
        response.type === 'error' &&
        response.error.code === ProtocolErrorCode.UnsupportedProtocolVersion
          ? device.scenario.device.protocolVersion
          : request.version,
      requestId: request.requestId,
      response,
    });
  };
}

function respond(device: DeterministicMockDevice, envelope: RequestEnvelope): Response {
  if (envelope.version.major !== protocolVersion.major) {
    return {
      type: 'error',
      error: {
        code: ProtocolErrorCode.UnsupportedProtocolVersion,
        supportedVersion: protocolVersion,
      },
    };
  }

  if (envelope.request.type === 'hello') {
    const handshake = device.handshake();
    if (handshake.kind === 'incompatible') {
      return {
        type: 'error',
        error: {
          code: ProtocolErrorCode.UnsupportedProtocolVersion,
          supportedVersion: device.scenario.device.protocolVersion,
        },
      };
    }
    return {
      type: 'hello',
      hello: {
        deviceId: stableBytes(handshake.device.deviceId, 16),
        productName: handshake.device.productName ?? 'FocusTimer',
        firmwareVersion: handshake.device.firmwareVersion,
        supportedVersion: handshake.device.protocolVersion,
        capabilities: [
          Capability.ReadStatus,
          Capability.ReadSessionPages,
          Capability.SetClockAnchor,
        ],
      },
    };
  }

  switch (envelope.request.type) {
    case 'getStatus': {
      const status = device.readStatus();
      return {
        type: 'status',
        status: {
          viewState: ViewState.Idle,
          preset: {
            id: status.selectedPreset.id,
            name: status.selectedPreset.name,
            plannedDurationMs: status.selectedPreset.durationMs,
          },
          remainingDurationMs: status.remainingDurationMs,
          journal: {
            epoch: stableBytes(status.journalEpoch, 8),
            ...(status.oldestSequence === null ? {} : { oldestSequence: status.oldestSequence }),
            ...(status.latestSequence === null ? {} : { latestSequence: status.latestSequence }),
            health: status.journalHealthy ? JournalHealth.Healthy : JournalHealth.Degraded,
          },
          clockKnown: status.clockKnown,
        },
      };
    }
    case 'setClockAnchor':
      return {
        type: 'clockAnchor',
        anchor: {
          acceptedUtcMs: envelope.request.anchor.utcMs,
          deviceMonotonicMsAtReceipt: 42,
        },
      };
    case 'getSessionPage': {
      const requestedEpoch =
        envelope.request.page.journalEpoch === undefined
          ? null
          : bytesKey(envelope.request.page.journalEpoch);
      const expectedEpoch = stableBytes(device.scenario.journal.epoch, 8);
      const pageResult = device.readSessionPage({
        journalEpoch:
          requestedEpoch === null || requestedEpoch === bytesKey(expectedEpoch)
            ? requestedEpoch === null
              ? null
              : device.scenario.journal.epoch
            : 'different-epoch',
        afterSequence: envelope.request.page.afterSequence,
        limit: envelope.request.page.limit,
      });
      if (pageResult.kind === 'malformed-response') {
        throw new TypeError('Malformed-response scenarios are handled by MockDeviceTransport');
      }
      const page = pageResult.page;
      return {
        type: 'sessionPage',
        page: {
          journalEpoch: stableBytes(page.journalEpoch, 8),
          ...(page.oldestSequence === null ? {} : { oldestSequence: page.oldestSequence }),
          ...(page.latestSequence === null ? {} : { latestSequence: page.latestSequence }),
          gap: page.gap,
          epochChanged: pageResult.kind === 'changed-epoch',
          records: page.records.map(toProtocolRecord),
          ...(page.continuationAfterSequence === null
            ? {}
            : { continuationAfterSequence: page.continuationAfterSequence }),
        },
      };
    }
    case 'unknown':
      return {
        type: 'error',
        error: {
          code: ProtocolErrorCode.UnsupportedMessage,
          failedMessageKind: envelope.request.messageKind,
        },
      };
  }
}

function toProtocolRecord(record: MockSessionRecord): SessionRecord {
  return {
    sequence: record.sequence,
    preset: {
      id: record.presetId,
      name: record.presetName,
      plannedDurationMs: record.plannedDurationMs,
    },
    activeDurationMs: record.activeDurationMs,
    outcome: record.outcome === 'completed' ? SessionOutcome.Completed : SessionOutcome.Cancelled,
    ...(record.startedAtUtcMs === null ? {} : { startedAtUtcMs: record.startedAtUtcMs }),
    ...(record.endedAtUtcMs === null ? {} : { endedAtUtcMs: record.endedAtUtcMs }),
  };
}

function stableBytes(value: string, length: number): Uint8Array {
  const output = new Uint8Array(length);
  for (let index = 0; index < value.length; index += 1) {
    const slot = index % length;
    output[slot] = ((output[slot] ?? 0) + value.charCodeAt(index) + index) & 0xff;
  }
  return output;
}

function bytesKey(bytes: Uint8Array): string {
  return [...bytes].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}
