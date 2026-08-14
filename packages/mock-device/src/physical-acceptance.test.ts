import { describe, expect, it } from 'vitest';

import { runProtocolFaultAcceptance } from '@focus-timer/device-client';

import { createProtocolMockTransport } from './protocol-responder';
import { getMockScenario } from './scenarios';

describe('physical protocol fault acceptance probe', () => {
  it('rejects bounded read-only faults and recovers the same session', async () => {
    const scenario = getMockScenario('paged');
    const transport = createProtocolMockTransport(scenario);
    await transport.connect(scenario.candidate, { timeoutMs: 1_000 });

    await expect(runProtocolFaultAcceptance(transport, { timeoutMs: 1_000 })).resolves.toEqual({
      duplicatePageWasStable: true,
      incompatibleVersionRejected: true,
      unknownCommandRejected: true,
      malformedPayloadRejected: true,
      validRequestRecovered: true,
    });
  });
});
