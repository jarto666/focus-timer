import {
  Capability,
  CatalogResult,
  JournalHealth,
  MAX_CAPABILITIES,
  MAX_CUSTOM_PRESETS,
  MAX_FIRMWARE_VERSION_BYTES,
  MAX_LOGICAL_MESSAGE_BYTES,
  MAX_PRESET_ID_BYTES,
  MAX_PRESET_NAME_BYTES,
  MAX_PRODUCT_NAME_BYTES,
  MAX_RECORDS_PER_PAGE,
  MAX_SAFE_PROTOCOL_INTEGER,
  MAX_TOTAL_PRESETS,
  ProtocolErrorCode,
  SessionOutcome,
  ViewState,
  type ClockAnchorResponse,
  type CatalogEntry,
  type DeviceEvent,
  type ErrorResponse,
  type EventEnvelope,
  type HelloResponse,
  type JournalStatus,
  type PresetSnapshot,
  type PresetCatalogResponse,
  type PresetCatalogResultEvent,
  type ProposePresetCatalogRequest,
  type ProposePresetCatalogResponse,
  type ProtocolVersion,
  type Request,
  type RequestEnvelope,
  type Response,
  type ResponseEnvelope,
  type SessionPageResponse,
  type SessionRecord,
  type StatusResponse,
} from './model';

const MAX_MAP_ENTRIES = 16;
const MAX_ARRAY_ITEMS = MAX_TOTAL_PRESETS;
const MAX_TEXT_BYTES = 32;
const MAX_NESTING_DEPTH = 6;
const MAX_UINT8 = 0xff;
const MAX_UINT32 = 0xffff_ffff;
const BUILT_IN_PRESET_IDS = new Set(['deep-work', 'focus', 'pomodoro', 'reading', 'quick-sprint']);

export type EncodeErrorCode = 'messageTooLarge' | 'invalidValue';

export class ProtocolEncodeError extends Error {
  public constructor(
    public readonly code: EncodeErrorCode,
    public readonly field?: string,
  ) {
    super(field === undefined ? code : `${code}: ${field}`);
    this.name = 'ProtocolEncodeError';
  }
}

export type DecodeErrorCode =
  | 'empty'
  | 'messageTooLarge'
  | 'truncated'
  | 'nonCanonical'
  | 'unsupportedType'
  | 'unexpectedType'
  | 'limitExceeded'
  | 'duplicateOrUnorderedKey'
  | 'invalidUtf8'
  | 'missingField'
  | 'invalidValue'
  | 'unsupportedMessage'
  | 'trailingData';

export class ProtocolDecodeError extends Error {
  public constructor(
    public readonly code: DecodeErrorCode,
    public readonly field?: number,
  ) {
    super(field === undefined ? code : `${code}: ${field}`);
    this.name = 'ProtocolDecodeError';
  }
}

export function encodeRequest(envelope: RequestEnvelope): Uint8Array {
  validateRequestEnvelope(envelope);
  const writer = new CborWriter();
  writeEnvelopePrefix(
    writer,
    envelope.version,
    envelope.requestId,
    requestMessageKind(envelope.request),
  );
  writeRequestPayload(writer, envelope.request);
  return writer.finish();
}

export function encodeResponse(envelope: ResponseEnvelope): Uint8Array {
  validateResponseEnvelope(envelope);
  const writer = new CborWriter();
  writeEnvelopePrefix(
    writer,
    envelope.version,
    envelope.requestId,
    responseMessageKind(envelope.response),
  );
  writeResponsePayload(writer, envelope.response);
  return writer.finish();
}

export function encodeEvent(envelope: EventEnvelope): Uint8Array {
  validateEventEnvelope(envelope);
  const writer = new CborWriter();
  writeEnvelopePrefix(writer, envelope.version, 0, eventMessageKind(envelope.event));
  writeEventPayload(writer, envelope.event);
  return writer.finish();
}

export function decodeRequest(input: Uint8Array): RequestEnvelope {
  const reader = new CborReader(input);
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let major: number | undefined;
  let minor: number | undefined;
  let requestId: number | undefined;
  let messageKind: number | undefined;
  let request: Request | undefined;

  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    switch (key) {
      case 0:
        major = reader.readUint8(0);
        break;
      case 1:
        minor = reader.readUint8(1);
        break;
      case 2:
        requestId = reader.readUint32(2);
        break;
      case 3:
        messageKind = reader.readUint();
        break;
      case 4:
        if (messageKind === undefined) {
          throw new ProtocolDecodeError('missingField', 3);
        }
        request = readRequestPayload(reader, messageKind, required(minor, 1));
        break;
      default:
        reader.skipValue(1);
    }
  }

  reader.finish();
  const envelope: RequestEnvelope = {
    version: {
      major: required(major, 0),
      minor: required(minor, 1),
    },
    requestId: required(requestId, 2),
    request: required(request, 4),
  };
  decodeValidate(() => validateRequestEnvelope(envelope));
  return envelope;
}

export function decodeResponse(input: Uint8Array): ResponseEnvelope {
  const reader = new CborReader(input);
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let major: number | undefined;
  let minor: number | undefined;
  let requestId: number | undefined;
  let messageKind: number | undefined;
  let response: Response | undefined;

  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    switch (key) {
      case 0:
        major = reader.readUint8(0);
        break;
      case 1:
        minor = reader.readUint8(1);
        break;
      case 2:
        requestId = reader.readUint32(2);
        break;
      case 3:
        messageKind = reader.readUint();
        break;
      case 4:
        if (messageKind === undefined) {
          throw new ProtocolDecodeError('missingField', 3);
        }
        response = readResponsePayload(reader, messageKind, required(minor, 1));
        break;
      default:
        reader.skipValue(1);
    }
  }

  reader.finish();
  const envelope: ResponseEnvelope = {
    version: {
      major: required(major, 0),
      minor: required(minor, 1),
    },
    requestId: required(requestId, 2),
    response: required(response, 4),
  };
  decodeValidate(() => validateResponseEnvelope(envelope));
  return envelope;
}

export function decodeEvent(input: Uint8Array): EventEnvelope {
  const reader = new CborReader(input);
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let major: number | undefined;
  let minor: number | undefined;
  let requestId: number | undefined;
  let messageKind: number | undefined;
  let event: DeviceEvent | undefined;
  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    if (key === 0) major = reader.readUint8(0);
    else if (key === 1) minor = reader.readUint8(1);
    else if (key === 2) requestId = reader.readUint32(2);
    else if (key === 3) messageKind = reader.readUint();
    else if (key === 4) {
      event = readEventPayload(reader, required(messageKind, 3), required(minor, 1));
    } else reader.skipValue(1);
  }
  reader.finish();
  if (required(requestId, 2) !== 0) throw new ProtocolDecodeError('invalidValue', 2);
  const envelope: EventEnvelope = {
    version: { major: required(major, 0), minor: required(minor, 1) },
    event: required(event, 4),
  };
  decodeValidate(() => validateEventEnvelope(envelope));
  return envelope;
}

function writeEnvelopePrefix(
  writer: CborWriter,
  version: ProtocolVersion,
  requestId: number,
  messageKind: number,
): void {
  writer.writeMap(5);
  writer.writeUint(0);
  writer.writeUint(version.major);
  writer.writeUint(1);
  writer.writeUint(version.minor);
  writer.writeUint(2);
  writer.writeUint(requestId);
  writer.writeUint(3);
  writer.writeUint(messageKind);
  writer.writeUint(4);
}

function writeRequestPayload(writer: CborWriter, request: Request): void {
  switch (request.type) {
    case 'hello':
    case 'getStatus':
    case 'getPresetCatalog':
    case 'unknown':
      writer.writeMap(0);
      break;
    case 'getSessionPage': {
      const { page } = request;
      writer.writeMap(page.journalEpoch === undefined ? 2 : 3);
      if (page.journalEpoch !== undefined) {
        writer.writeUint(0);
        writer.writeBytes(page.journalEpoch);
      }
      writer.writeUint(1);
      writer.writeUint(page.afterSequence);
      writer.writeUint(2);
      writer.writeUint(page.limit);
      break;
    }
    case 'setClockAnchor':
      writer.writeMap(1);
      writer.writeUint(0);
      writer.writeUint(request.anchor.utcMs);
      break;
    case 'proposePresetCatalog':
      writeCatalogProposalRequest(writer, request.proposal);
      break;
  }
}

function writeResponsePayload(writer: CborWriter, response: Response): void {
  switch (response.type) {
    case 'hello':
      writeHello(writer, response.hello);
      break;
    case 'status':
      writeStatus(writer, response.status);
      break;
    case 'sessionPage':
      writeSessionPage(writer, response.page);
      break;
    case 'clockAnchor':
      writeClockAnchor(writer, response.anchor);
      break;
    case 'presetCatalog':
      writePresetCatalog(writer, response.catalog);
      break;
    case 'proposePresetCatalog':
      writeCatalogProposalResponse(writer, response.proposal);
      break;
    case 'error':
      writeError(writer, response.error);
      break;
  }
}

function writeEventPayload(writer: CborWriter, event: DeviceEvent): void {
  if (event.type === 'liveStatus') writeStatus(writer, event.status);
  else writeCatalogResult(writer, event.result);
}

function writeHello(writer: CborWriter, hello: HelloResponse): void {
  writer.writeMap(6);
  writer.writeUint(0);
  writer.writeBytes(hello.deviceId);
  writer.writeUint(1);
  writer.writeText(hello.productName);
  writer.writeUint(2);
  writer.writeText(hello.firmwareVersion);
  writer.writeUint(3);
  writer.writeUint(hello.supportedVersion.major);
  writer.writeUint(4);
  writer.writeUint(hello.supportedVersion.minor);
  writer.writeUint(5);
  writer.writeArray(hello.capabilities.length);
  for (const capability of hello.capabilities) {
    writer.writeUint(capability);
  }
}

function writeStatus(writer: CborWriter, status: StatusResponse): void {
  writer.writeMap(status.statusEpoch === undefined ? 5 : 7);
  writer.writeUint(0);
  writer.writeUint(status.viewState);
  writer.writeUint(1);
  writePreset(writer, status.preset);
  writer.writeUint(2);
  writer.writeUint(status.remainingDurationMs);
  writer.writeUint(3);
  writeJournalStatus(writer, status.journal);
  writer.writeUint(4);
  writer.writeBoolean(status.clockKnown);
  if (status.statusEpoch !== undefined) {
    writer.writeUint(5);
    writer.writeBytes(status.statusEpoch);
  }
  if (status.statusRevision !== undefined) {
    writer.writeUint(6);
    writer.writeUint(status.statusRevision);
  }
}

function writeCatalogEntry(writer: CborWriter, entry: CatalogEntry): void {
  writer.writeMap(4);
  writer.writeUint(0);
  writer.writeText(entry.id);
  writer.writeUint(1);
  writer.writeText(entry.name);
  writer.writeUint(2);
  writer.writeUint(entry.plannedDurationMs);
  writer.writeUint(3);
  writer.writeBoolean(entry.builtIn);
}

function writePresetCatalog(writer: CborWriter, catalog: PresetCatalogResponse): void {
  writer.writeMap(2);
  writer.writeUint(0);
  writer.writeUint(catalog.revision);
  writer.writeUint(1);
  writer.writeArray(catalog.entries.length);
  for (const entry of catalog.entries) writeCatalogEntry(writer, entry);
}

function writeCatalogProposalRequest(
  writer: CborWriter,
  proposal: ProposePresetCatalogRequest,
): void {
  writer.writeMap(3);
  writer.writeUint(0);
  writer.writeUint(proposal.expectedRevision);
  writer.writeUint(1);
  writer.writeUint(proposal.proposalId);
  writer.writeUint(2);
  writer.writeArray(proposal.customEntries.length);
  for (const preset of proposal.customEntries) writePreset(writer, preset);
}

function writeCatalogProposalResponse(
  writer: CborWriter,
  proposal: ProposePresetCatalogResponse,
): void {
  writer.writeMap(2);
  writer.writeUint(0);
  writer.writeUint(proposal.proposalId);
  writer.writeUint(1);
  writer.writeUint(proposal.expiresInMs);
}

function writeCatalogResult(writer: CborWriter, result: PresetCatalogResultEvent): void {
  writer.writeMap(result.catalogRevision === undefined ? 2 : 3);
  writer.writeUint(0);
  writer.writeUint(result.proposalId);
  writer.writeUint(1);
  writer.writeUint(result.result);
  if (result.catalogRevision !== undefined) {
    writer.writeUint(2);
    writer.writeUint(result.catalogRevision);
  }
}

function writePreset(writer: CborWriter, preset: PresetSnapshot): void {
  writer.writeMap(3);
  writer.writeUint(0);
  writer.writeText(preset.id);
  writer.writeUint(1);
  writer.writeText(preset.name);
  writer.writeUint(2);
  writer.writeUint(preset.plannedDurationMs);
}

function writeJournalStatus(writer: CborWriter, journal: JournalStatus): void {
  const hasBounds = journal.oldestSequence !== undefined;
  writer.writeMap(hasBounds ? 4 : 2);
  writer.writeUint(0);
  writer.writeBytes(journal.epoch);
  if (journal.oldestSequence !== undefined) {
    writer.writeUint(1);
    writer.writeUint(journal.oldestSequence);
  }
  if (journal.latestSequence !== undefined) {
    writer.writeUint(2);
    writer.writeUint(journal.latestSequence);
  }
  writer.writeUint(3);
  writer.writeUint(journal.health);
}

function writeSessionPage(writer: CborWriter, page: SessionPageResponse): void {
  let fieldCount = 4;
  if (page.oldestSequence !== undefined) fieldCount += 2;
  if (page.continuationAfterSequence !== undefined) fieldCount += 1;
  writer.writeMap(fieldCount);
  writer.writeUint(0);
  writer.writeBytes(page.journalEpoch);
  if (page.oldestSequence !== undefined) {
    writer.writeUint(1);
    writer.writeUint(page.oldestSequence);
  }
  if (page.latestSequence !== undefined) {
    writer.writeUint(2);
    writer.writeUint(page.latestSequence);
  }
  writer.writeUint(3);
  writer.writeBoolean(page.gap);
  writer.writeUint(4);
  writer.writeBoolean(page.epochChanged);
  writer.writeUint(5);
  writer.writeArray(page.records.length);
  for (const record of page.records) writeSessionRecord(writer, record);
  if (page.continuationAfterSequence !== undefined) {
    writer.writeUint(6);
    writer.writeUint(page.continuationAfterSequence);
  }
}

function writeSessionRecord(writer: CborWriter, record: SessionRecord): void {
  let fieldCount = 4;
  if (record.startedAtUtcMs !== undefined) fieldCount += 1;
  if (record.endedAtUtcMs !== undefined) fieldCount += 1;
  writer.writeMap(fieldCount);
  writer.writeUint(0);
  writer.writeUint(record.sequence);
  writer.writeUint(1);
  writePreset(writer, record.preset);
  writer.writeUint(2);
  writer.writeUint(record.activeDurationMs);
  writer.writeUint(3);
  writer.writeUint(record.outcome);
  if (record.startedAtUtcMs !== undefined) {
    writer.writeUint(4);
    writer.writeUint(record.startedAtUtcMs);
  }
  if (record.endedAtUtcMs !== undefined) {
    writer.writeUint(5);
    writer.writeUint(record.endedAtUtcMs);
  }
}

function writeClockAnchor(writer: CborWriter, anchor: ClockAnchorResponse): void {
  writer.writeMap(2);
  writer.writeUint(0);
  writer.writeUint(anchor.acceptedUtcMs);
  writer.writeUint(1);
  writer.writeUint(anchor.deviceMonotonicMsAtReceipt);
}

function writeError(writer: CborWriter, error: ErrorResponse): void {
  let fieldCount = 1;
  if (error.failedMessageKind !== undefined) fieldCount += 1;
  if (error.fieldId !== undefined) fieldCount += 1;
  if (error.supportedVersion !== undefined) fieldCount += 2;
  writer.writeMap(fieldCount);
  writer.writeUint(0);
  writer.writeUint(error.code);
  if (error.failedMessageKind !== undefined) {
    writer.writeUint(1);
    writer.writeUint(error.failedMessageKind);
  }
  if (error.fieldId !== undefined) {
    writer.writeUint(2);
    writer.writeUint(error.fieldId);
  }
  if (error.supportedVersion !== undefined) {
    writer.writeUint(3);
    writer.writeUint(error.supportedVersion.major);
    writer.writeUint(4);
    writer.writeUint(error.supportedVersion.minor);
  }
}

function readRequestPayload(
  reader: CborReader,
  messageKind: number,
  protocolMinor: number,
): Request {
  switch (messageKind) {
    case 1:
      reader.readEmptyMap();
      return { type: 'hello' };
    case 3:
      reader.readEmptyMap();
      return { type: 'getStatus' };
    case 5: {
      const length = reader.readMapLength();
      let previousKey: number | undefined;
      let journalEpoch: Uint8Array | undefined;
      let afterSequence: number | undefined;
      let limit: number | undefined;
      for (let index = 0; index < length; index += 1) {
        const key = reader.readMapKey(previousKey);
        previousKey = key;
        switch (key) {
          case 0:
            journalEpoch = reader.readBytesExact(8, 0);
            break;
          case 1:
            afterSequence = reader.readUint();
            break;
          case 2:
            limit = reader.readUint8(2);
            break;
          default:
            reader.skipValue(2);
        }
      }
      return {
        type: 'getSessionPage',
        page: {
          ...(journalEpoch === undefined ? {} : { journalEpoch }),
          afterSequence: required(afterSequence, 1),
          limit: required(limit, 2),
        },
      };
    }
    case 7: {
      const length = reader.readMapLength();
      let previousKey: number | undefined;
      let utcMs: number | undefined;
      for (let index = 0; index < length; index += 1) {
        const key = reader.readMapKey(previousKey);
        previousKey = key;
        if (key === 0) utcMs = reader.readUint();
        else reader.skipValue(2);
      }
      return { type: 'setClockAnchor', anchor: { utcMs: required(utcMs, 0) } };
    }
    case 9:
      if (protocolMinor < 1) break;
      reader.readEmptyMap();
      return { type: 'getPresetCatalog' };
    case 11:
      if (protocolMinor < 1) break;
      return { type: 'proposePresetCatalog', proposal: readCatalogProposalRequest(reader) };
    default:
      break;
  }
  reader.skipMap(2);
  return { type: 'unknown', messageKind };
}

function readResponsePayload(
  reader: CborReader,
  messageKind: number,
  protocolMinor: number,
): Response {
  switch (messageKind) {
    case 2:
      return { type: 'hello', hello: readHello(reader) };
    case 4:
      return { type: 'status', status: readStatus(reader) };
    case 6:
      return { type: 'sessionPage', page: readSessionPage(reader) };
    case 8:
      return { type: 'clockAnchor', anchor: readClockAnchor(reader) };
    case 10:
      if (protocolMinor >= 1) return { type: 'presetCatalog', catalog: readPresetCatalog(reader) };
      break;
    case 12:
      if (protocolMinor >= 1)
        return { type: 'proposePresetCatalog', proposal: readCatalogProposalResponse(reader) };
      break;
    case 255:
      return { type: 'error', error: readError(reader) };
    default:
      break;
  }
  throw new ProtocolDecodeError('unsupportedMessage', messageKind);
}

function readEventPayload(
  reader: CborReader,
  messageKind: number,
  protocolMinor: number,
): DeviceEvent {
  if (protocolMinor >= 1 && messageKind === 13)
    return { type: 'liveStatus', status: readStatus(reader) };
  if (protocolMinor >= 1 && messageKind === 14)
    return { type: 'presetCatalogResult', result: readCatalogResult(reader) };
  throw new ProtocolDecodeError('unsupportedMessage', messageKind);
}

function readHello(reader: CborReader): HelloResponse {
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let deviceId: Uint8Array | undefined;
  let productName: string | undefined;
  let firmwareVersion: string | undefined;
  let major: number | undefined;
  let minor: number | undefined;
  let capabilities: Capability[] | undefined;
  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    switch (key) {
      case 0:
        deviceId = reader.readBytesExact(16, 0);
        break;
      case 1:
        productName = reader.readText(MAX_PRODUCT_NAME_BYTES, 1);
        break;
      case 2:
        firmwareVersion = reader.readText(MAX_FIRMWARE_VERSION_BYTES, 2);
        break;
      case 3:
        major = reader.readUint8(3);
        break;
      case 4:
        minor = reader.readUint8(4);
        break;
      case 5:
        capabilities = readCapabilities(reader);
        break;
      default:
        reader.skipValue(2);
    }
  }
  return {
    deviceId: required(deviceId, 0),
    productName: required(productName, 1),
    firmwareVersion: required(firmwareVersion, 2),
    supportedVersion: { major: required(major, 3), minor: required(minor, 4) },
    capabilities: required(capabilities, 5),
  };
}

function readCapabilities(reader: CborReader): Capability[] {
  const length = reader.readArrayLength();
  const capabilities: Capability[] = [];
  let previous: number | undefined;
  for (let index = 0; index < length; index += 1) {
    const raw = reader.readUint();
    if (previous !== undefined && raw <= previous) throw new ProtocolDecodeError('invalidValue', 5);
    previous = raw;
    if (isCapability(raw)) capabilities.push(raw);
  }
  return capabilities;
}

function readStatus(reader: CborReader): StatusResponse {
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let viewState: ViewState | undefined;
  let preset: PresetSnapshot | undefined;
  let remainingDurationMs: number | undefined;
  let journal: JournalStatus | undefined;
  let clockKnown: boolean | undefined;
  let statusEpoch: Uint8Array | undefined;
  let statusRevision: number | undefined;
  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    switch (key) {
      case 0: {
        const raw = reader.readUint();
        if (!isViewState(raw)) throw new ProtocolDecodeError('invalidValue', 0);
        viewState = raw;
        break;
      }
      case 1:
        preset = readPreset(reader);
        break;
      case 2:
        remainingDurationMs = reader.readUint32(2);
        break;
      case 3:
        journal = readJournalStatus(reader);
        break;
      case 4:
        clockKnown = reader.readBoolean();
        break;
      case 5:
        statusEpoch = reader.readBytesExact(8, 5);
        break;
      case 6:
        statusRevision = reader.readUint();
        break;
      default:
        reader.skipValue(2);
    }
  }
  return {
    viewState: required(viewState, 0),
    preset: required(preset, 1),
    remainingDurationMs: required(remainingDurationMs, 2),
    journal: required(journal, 3),
    clockKnown: required(clockKnown, 4),
    ...(statusEpoch === undefined ? {} : { statusEpoch }),
    ...(statusRevision === undefined ? {} : { statusRevision }),
  };
}

function readCatalogEntry(reader: CborReader): CatalogEntry {
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let id: string | undefined;
  let name: string | undefined;
  let plannedDurationMs: number | undefined;
  let builtIn: boolean | undefined;
  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    if (key === 0) id = reader.readText(MAX_PRESET_ID_BYTES, 0);
    else if (key === 1) name = reader.readText(MAX_PRESET_NAME_BYTES, 1);
    else if (key === 2) plannedDurationMs = reader.readUint32(2);
    else if (key === 3) builtIn = reader.readBoolean();
    else reader.skipValue(3);
  }
  return {
    id: required(id, 0),
    name: required(name, 1),
    plannedDurationMs: required(plannedDurationMs, 2),
    builtIn: required(builtIn, 3),
  };
}

function readPresetCatalog(reader: CborReader): PresetCatalogResponse {
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let revision: number | undefined;
  let entries: CatalogEntry[] | undefined;
  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    if (key === 0) revision = reader.readUint();
    else if (key === 1) {
      const count = reader.readArrayLength();
      entries = [];
      for (let item = 0; item < count; item += 1) entries.push(readCatalogEntry(reader));
    } else reader.skipValue(2);
  }
  return { revision: required(revision, 0), entries: required(entries, 1) };
}

function readCatalogProposalRequest(reader: CborReader): ProposePresetCatalogRequest {
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let expectedRevision: number | undefined;
  let proposalId: number | undefined;
  let customEntries: PresetSnapshot[] | undefined;
  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    if (key === 0) expectedRevision = reader.readUint();
    else if (key === 1) proposalId = reader.readUint32(1);
    else if (key === 2) {
      const count = reader.readArrayLength();
      customEntries = [];
      for (let item = 0; item < count; item += 1) customEntries.push(readPreset(reader));
    } else reader.skipValue(2);
  }
  return {
    expectedRevision: required(expectedRevision, 0),
    proposalId: required(proposalId, 1),
    customEntries: required(customEntries, 2),
  };
}

function readCatalogProposalResponse(reader: CborReader): ProposePresetCatalogResponse {
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let proposalId: number | undefined;
  let expiresInMs: number | undefined;
  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    if (key === 0) proposalId = reader.readUint32(0);
    else if (key === 1) expiresInMs = reader.readUint32(1);
    else reader.skipValue(2);
  }
  return { proposalId: required(proposalId, 0), expiresInMs: required(expiresInMs, 1) };
}

function readCatalogResult(reader: CborReader): PresetCatalogResultEvent {
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let proposalId: number | undefined;
  let result: CatalogResult | undefined;
  let catalogRevision: number | undefined;
  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    if (key === 0) proposalId = reader.readUint32(0);
    else if (key === 1) {
      const raw = reader.readUint();
      if (!isCatalogResult(raw)) throw new ProtocolDecodeError('invalidValue', 1);
      result = raw;
    } else if (key === 2) catalogRevision = reader.readUint();
    else reader.skipValue(2);
  }
  return {
    proposalId: required(proposalId, 0),
    result: required(result, 1),
    ...(catalogRevision === undefined ? {} : { catalogRevision }),
  };
}

function readPreset(reader: CborReader): PresetSnapshot {
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let id: string | undefined;
  let name: string | undefined;
  let plannedDurationMs: number | undefined;
  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    if (key === 0) id = reader.readText(MAX_PRESET_ID_BYTES, 0);
    else if (key === 1) name = reader.readText(MAX_PRESET_NAME_BYTES, 1);
    else if (key === 2) plannedDurationMs = reader.readUint32(2);
    else reader.skipValue(3);
  }
  return {
    id: required(id, 0),
    name: required(name, 1),
    plannedDurationMs: required(plannedDurationMs, 2),
  };
}

function readJournalStatus(reader: CborReader): JournalStatus {
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let epoch: Uint8Array | undefined;
  let oldestSequence: number | undefined;
  let latestSequence: number | undefined;
  let health: JournalHealth | undefined;
  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    if (key === 0) epoch = reader.readBytesExact(8, 0);
    else if (key === 1) oldestSequence = reader.readUint();
    else if (key === 2) latestSequence = reader.readUint();
    else if (key === 3) {
      const raw = reader.readUint();
      if (!isJournalHealth(raw)) throw new ProtocolDecodeError('invalidValue', 3);
      health = raw;
    } else reader.skipValue(3);
  }
  return {
    epoch: required(epoch, 0),
    ...(oldestSequence === undefined ? {} : { oldestSequence }),
    ...(latestSequence === undefined ? {} : { latestSequence }),
    health: required(health, 3),
  };
}

function readSessionPage(reader: CborReader): SessionPageResponse {
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let journalEpoch: Uint8Array | undefined;
  let oldestSequence: number | undefined;
  let latestSequence: number | undefined;
  let gap: boolean | undefined;
  let epochChanged: boolean | undefined;
  let records: SessionRecord[] | undefined;
  let continuationAfterSequence: number | undefined;
  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    if (key === 0) journalEpoch = reader.readBytesExact(8, 0);
    else if (key === 1) oldestSequence = reader.readUint();
    else if (key === 2) latestSequence = reader.readUint();
    else if (key === 3) gap = reader.readBoolean();
    else if (key === 4) epochChanged = reader.readBoolean();
    else if (key === 5) records = readSessionRecords(reader);
    else if (key === 6) continuationAfterSequence = reader.readUint();
    else reader.skipValue(2);
  }
  return {
    journalEpoch: required(journalEpoch, 0),
    ...(oldestSequence === undefined ? {} : { oldestSequence }),
    ...(latestSequence === undefined ? {} : { latestSequence }),
    gap: required(gap, 3),
    epochChanged: required(epochChanged, 4),
    records: required(records, 5),
    ...(continuationAfterSequence === undefined ? {} : { continuationAfterSequence }),
  };
}

function readSessionRecords(reader: CborReader): SessionRecord[] {
  const length = reader.readArrayLength();
  const records: SessionRecord[] = [];
  for (let index = 0; index < length; index += 1) records.push(readSessionRecord(reader));
  return records;
}

function readSessionRecord(reader: CborReader): SessionRecord {
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let sequence: number | undefined;
  let preset: PresetSnapshot | undefined;
  let activeDurationMs: number | undefined;
  let outcome: SessionOutcome | undefined;
  let startedAtUtcMs: number | undefined;
  let endedAtUtcMs: number | undefined;
  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    if (key === 0) sequence = reader.readUint();
    else if (key === 1) preset = readPreset(reader);
    else if (key === 2) activeDurationMs = reader.readUint32(2);
    else if (key === 3) {
      const raw = reader.readUint();
      if (!isSessionOutcome(raw)) throw new ProtocolDecodeError('invalidValue', 3);
      outcome = raw;
    } else if (key === 4) startedAtUtcMs = reader.readUint();
    else if (key === 5) endedAtUtcMs = reader.readUint();
    else reader.skipValue(4);
  }
  return {
    sequence: required(sequence, 0),
    preset: required(preset, 1),
    activeDurationMs: required(activeDurationMs, 2),
    outcome: required(outcome, 3),
    ...(startedAtUtcMs === undefined ? {} : { startedAtUtcMs }),
    ...(endedAtUtcMs === undefined ? {} : { endedAtUtcMs }),
  };
}

function readClockAnchor(reader: CborReader): ClockAnchorResponse {
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let acceptedUtcMs: number | undefined;
  let deviceMonotonicMsAtReceipt: number | undefined;
  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    if (key === 0) acceptedUtcMs = reader.readUint();
    else if (key === 1) deviceMonotonicMsAtReceipt = reader.readUint();
    else reader.skipValue(2);
  }
  return {
    acceptedUtcMs: required(acceptedUtcMs, 0),
    deviceMonotonicMsAtReceipt: required(deviceMonotonicMsAtReceipt, 1),
  };
}

function readError(reader: CborReader): ErrorResponse {
  const length = reader.readMapLength();
  let previousKey: number | undefined;
  let code: ProtocolErrorCode | undefined;
  let failedMessageKind: number | undefined;
  let fieldId: number | undefined;
  let major: number | undefined;
  let minor: number | undefined;
  for (let index = 0; index < length; index += 1) {
    const key = reader.readMapKey(previousKey);
    previousKey = key;
    if (key === 0) {
      const raw = reader.readUint();
      if (!isProtocolErrorCode(raw)) throw new ProtocolDecodeError('invalidValue', 0);
      code = raw;
    } else if (key === 1) failedMessageKind = reader.readUint();
    else if (key === 2) fieldId = reader.readUint();
    else if (key === 3) major = reader.readUint8(3);
    else if (key === 4) minor = reader.readUint8(4);
    else reader.skipValue(2);
  }
  if ((major === undefined) !== (minor === undefined))
    throw new ProtocolDecodeError('invalidValue', 3);
  return {
    code: required(code, 0),
    ...(failedMessageKind === undefined ? {} : { failedMessageKind }),
    ...(fieldId === undefined ? {} : { fieldId }),
    ...(major === undefined || minor === undefined ? {} : { supportedVersion: { major, minor } }),
  };
}

function validateRequestEnvelope(envelope: RequestEnvelope): void {
  validateVersion(envelope.version);
  validateRequestId(envelope.requestId);
  switch (envelope.request.type) {
    case 'hello':
    case 'getStatus':
    case 'getPresetCatalog':
      break;
    case 'unknown':
      validateSafeUint(envelope.request.messageKind, 'messageKind');
      if (envelope.request.messageKind === 0) invalid('messageKind');
      break;
    case 'getSessionPage': {
      const { page } = envelope.request;
      validateSafeUint(page.afterSequence, 'afterSequence');
      validateUint(page.limit, MAX_UINT8, 'limit');
      if (page.limit === 0 || page.limit > MAX_RECORDS_PER_PAGE) invalid('limit');
      if (page.journalEpoch === undefined && page.afterSequence !== 0) invalid('journalEpoch');
      if (page.journalEpoch !== undefined) validateBytes(page.journalEpoch, 8, 'journalEpoch');
      break;
    }
    case 'setClockAnchor':
      validateSafeUint(envelope.request.anchor.utcMs, 'utcMs');
      break;
    case 'proposePresetCatalog':
      validateCatalogProposal(envelope.request.proposal);
      break;
  }
}

function validateResponseEnvelope(envelope: ResponseEnvelope): void {
  validateVersion(envelope.version);
  validateRequestId(envelope.requestId);
  switch (envelope.response.type) {
    case 'hello':
      validateHello(envelope.response.hello);
      break;
    case 'status':
      validateStatus(envelope.response.status);
      break;
    case 'sessionPage':
      validatePage(envelope.response.page);
      break;
    case 'clockAnchor':
      validateSafeUint(envelope.response.anchor.acceptedUtcMs, 'acceptedUtcMs');
      validateSafeUint(
        envelope.response.anchor.deviceMonotonicMsAtReceipt,
        'deviceMonotonicMsAtReceipt',
      );
      break;
    case 'presetCatalog':
      validateCatalog(envelope.response.catalog);
      break;
    case 'proposePresetCatalog':
      validateNonzeroUint32(envelope.response.proposal.proposalId, 'proposalId');
      validateUint(envelope.response.proposal.expiresInMs, MAX_UINT32, 'expiresInMs');
      if (envelope.response.proposal.expiresInMs === 0) invalid('expiresInMs');
      break;
    case 'error':
      validateError(envelope.response.error);
      break;
  }
}

function validateEventEnvelope(envelope: EventEnvelope): void {
  validateVersion(envelope.version);
  if (envelope.version.minor < 1) invalid('protocolMinor');
  if (envelope.event.type === 'liveStatus') {
    validateStatus(envelope.event.status);
    if (envelope.event.status.statusEpoch === undefined) invalid('statusEpoch');
  } else validateCatalogResult(envelope.event.result);
}

function validateVersion(version: ProtocolVersion): void {
  validateUint(version.major, MAX_UINT8, 'protocolMajor');
  validateUint(version.minor, MAX_UINT8, 'protocolMinor');
}

function validateRequestId(requestId: number): void {
  validateUint(requestId, MAX_UINT32, 'requestId');
  if (requestId === 0) invalid('requestId');
}

function validateHello(hello: HelloResponse): void {
  validateBytes(hello.deviceId, 16, 'deviceId');
  validateText(hello.productName, MAX_PRODUCT_NAME_BYTES, 'productName');
  validateText(hello.firmwareVersion, MAX_FIRMWARE_VERSION_BYTES, 'firmwareVersion');
  validateVersion(hello.supportedVersion);
  if (hello.capabilities.length > MAX_CAPABILITIES) invalid('capabilities');
  let previous: number | undefined;
  for (const capability of hello.capabilities) {
    if (!isCapability(capability) || (previous !== undefined && capability <= previous)) {
      invalid('capabilities');
    }
    previous = capability;
  }
}

function validateStatus(status: StatusResponse): void {
  if (!isViewState(status.viewState)) invalid('viewState');
  validatePreset(status.preset);
  validateUint(status.remainingDurationMs, MAX_UINT32, 'remainingDurationMs');
  validateBytes(status.journal.epoch, 8, 'journalEpoch');
  if (!isJournalHealth(status.journal.health)) invalid('journalHealth');
  validateBounds(status.journal.oldestSequence, status.journal.latestSequence);
  if ((status.statusEpoch === undefined) !== (status.statusRevision === undefined)) {
    invalid('statusEpoch');
  }
  if (status.statusEpoch !== undefined) validateBytes(status.statusEpoch, 8, 'statusEpoch');
  if (status.statusRevision !== undefined) {
    validateSafeUint(status.statusRevision, 'statusRevision');
    if (status.statusRevision === 0) invalid('statusRevision');
  }
}

function validateCatalog(catalog: PresetCatalogResponse): void {
  validateSafeUint(catalog.revision, 'catalogRevision');
  if (catalog.entries.length < 5 || catalog.entries.length > MAX_TOTAL_PRESETS) {
    invalid('catalogEntries');
  }
  let customSeen = false;
  let builtInCount = 0;
  const ids = new Set<string>();
  for (const entry of catalog.entries) {
    validatePreset(entry);
    if (ids.has(entry.id)) invalid('presetId');
    ids.add(entry.id);
    if (entry.builtIn) {
      if (customSeen) invalid('builtIn');
      builtInCount += 1;
    } else {
      customSeen = true;
      validateCustomPreset(entry);
    }
  }
  if (builtInCount !== 5 || catalog.entries.length - builtInCount > MAX_CUSTOM_PRESETS) {
    invalid('builtIn');
  }
}

function validateCatalogProposal(proposal: ProposePresetCatalogRequest): void {
  validateSafeUint(proposal.expectedRevision, 'expectedRevision');
  validateNonzeroUint32(proposal.proposalId, 'proposalId');
  if (proposal.customEntries.length > MAX_CUSTOM_PRESETS) invalid('customEntries');
  const ids = new Set<string>();
  for (const preset of proposal.customEntries) {
    validateCustomPreset(preset);
    if (BUILT_IN_PRESET_IDS.has(preset.id)) invalid('presetId');
    if (ids.has(preset.id)) invalid('presetId');
    ids.add(preset.id);
  }
}

function validateCustomPreset(preset: PresetSnapshot): void {
  validatePreset(preset);
  if (
    preset.plannedDurationMs < 60_000 ||
    preset.plannedDurationMs > 43_200_000 ||
    preset.plannedDurationMs % 60_000 !== 0
  ) {
    invalid('plannedDurationMs');
  }
}

function validateCatalogResult(result: PresetCatalogResultEvent): void {
  validateNonzeroUint32(result.proposalId, 'proposalId');
  if (!isCatalogResult(result.result)) invalid('result');
  if (result.result === CatalogResult.Committed) {
    if (result.catalogRevision === undefined || result.catalogRevision === 0) {
      invalid('catalogRevision');
    }
    validateSafeUint(result.catalogRevision, 'catalogRevision');
  } else if (result.catalogRevision !== undefined) invalid('catalogRevision');
}

function validateNonzeroUint32(value: number, field: string): void {
  validateUint(value, MAX_UINT32, field);
  if (value === 0) invalid(field);
}

function validatePreset(preset: PresetSnapshot): void {
  validateText(preset.id, MAX_PRESET_ID_BYTES, 'presetId');
  validateText(preset.name, MAX_PRESET_NAME_BYTES, 'presetName');
  validateUint(preset.plannedDurationMs, MAX_UINT32, 'plannedDurationMs');
  if (preset.plannedDurationMs === 0) invalid('plannedDurationMs');
}

function validatePage(page: SessionPageResponse): void {
  validateBytes(page.journalEpoch, 8, 'journalEpoch');
  validateBounds(page.oldestSequence, page.latestSequence);
  if (page.records.length > MAX_RECORDS_PER_PAGE) invalid('records');
  if (page.gap && page.epochChanged) invalid('gap');
  if (
    page.oldestSequence === undefined &&
    (page.records.length > 0 || page.continuationAfterSequence !== undefined)
  ) {
    invalid('records');
  }
  let previous: number | undefined;
  for (const record of page.records) {
    validateRecord(record);
    if (previous !== undefined && record.sequence <= previous) invalid('sequence');
    if (page.oldestSequence !== undefined && record.sequence < page.oldestSequence)
      invalid('sequence');
    if (page.latestSequence !== undefined && record.sequence > page.latestSequence)
      invalid('sequence');
    previous = record.sequence;
  }
  if (page.continuationAfterSequence !== undefined) {
    validateSafeUint(page.continuationAfterSequence, 'continuationAfterSequence');
    if (
      previous !== page.continuationAfterSequence ||
      page.latestSequence === undefined ||
      page.continuationAfterSequence >= page.latestSequence
    ) {
      invalid('continuationAfterSequence');
    }
  }
}

function validateRecord(record: SessionRecord): void {
  validateSafeUint(record.sequence, 'sequence');
  if (record.sequence === 0) invalid('sequence');
  validatePreset(record.preset);
  validateUint(record.activeDurationMs, MAX_UINT32, 'activeDurationMs');
  if (record.activeDurationMs > record.preset.plannedDurationMs) invalid('activeDurationMs');
  if (!isSessionOutcome(record.outcome)) invalid('outcome');
  if (record.startedAtUtcMs !== undefined)
    validateSafeUint(record.startedAtUtcMs, 'startedAtUtcMs');
  if (record.endedAtUtcMs !== undefined) validateSafeUint(record.endedAtUtcMs, 'endedAtUtcMs');
  if (
    record.startedAtUtcMs !== undefined &&
    record.endedAtUtcMs !== undefined &&
    record.startedAtUtcMs > record.endedAtUtcMs
  ) {
    invalid('endedAtUtcMs');
  }
}

function validateBounds(oldest: number | undefined, latest: number | undefined): void {
  if (oldest === undefined && latest === undefined) return;
  if (oldest === undefined || latest === undefined || oldest === 0 || oldest > latest) {
    invalid('journalBounds');
  }
  validateSafeUint(oldest, 'oldestSequence');
  validateSafeUint(latest, 'latestSequence');
}

function validateError(error: ErrorResponse): void {
  if (!isProtocolErrorCode(error.code)) invalid('code');
  if (error.failedMessageKind !== undefined)
    validateSafeUint(error.failedMessageKind, 'failedMessageKind');
  if (error.fieldId !== undefined) validateSafeUint(error.fieldId, 'fieldId');
  if (error.code === ProtocolErrorCode.UnsupportedProtocolVersion) {
    if (error.supportedVersion === undefined) invalid('supportedVersion');
    validateVersion(error.supportedVersion);
  } else if (error.supportedVersion !== undefined) invalid('supportedVersion');
}

function validateText(value: string, maximumBytes: number, field: string): void {
  const encoded = encodeUtf8(value);
  if (encoded.length === 0 || encoded.length > maximumBytes) invalid(field);
}

function validateBytes(value: Uint8Array, length: number, field: string): void {
  if (!(value instanceof Uint8Array) || value.length !== length) invalid(field);
}

function validateUint(value: number, maximum: number, field: string): void {
  if (!Number.isSafeInteger(value) || value < 0 || value > maximum) invalid(field);
}

function validateSafeUint(value: number, field: string): void {
  validateUint(value, MAX_SAFE_PROTOCOL_INTEGER, field);
}

function invalid(field: string): never {
  throw new ProtocolEncodeError('invalidValue', field);
}

function decodeValidate(validation: () => void): void {
  try {
    validation();
  } catch (error) {
    if (error instanceof ProtocolEncodeError) throw new ProtocolDecodeError('invalidValue', 4);
    throw error;
  }
}

function required<T>(value: T | undefined, field: number): T {
  if (value === undefined) throw new ProtocolDecodeError('missingField', field);
  return value;
}

function requestMessageKind(request: Request): number {
  switch (request.type) {
    case 'hello':
      return 1;
    case 'getStatus':
      return 3;
    case 'getSessionPage':
      return 5;
    case 'setClockAnchor':
      return 7;
    case 'getPresetCatalog':
      return 9;
    case 'proposePresetCatalog':
      return 11;
    case 'unknown':
      return request.messageKind;
  }
}

function responseMessageKind(response: Response): number {
  switch (response.type) {
    case 'hello':
      return 2;
    case 'status':
      return 4;
    case 'sessionPage':
      return 6;
    case 'clockAnchor':
      return 8;
    case 'presetCatalog':
      return 10;
    case 'proposePresetCatalog':
      return 12;
    case 'error':
      return 255;
  }
}

function eventMessageKind(event: DeviceEvent): number {
  return event.type === 'liveStatus' ? 13 : 14;
}

function isCapability(value: number): value is Capability {
  return (
    value === Capability.ReadStatus ||
    value === Capability.ReadSessionPages ||
    value === Capability.SetClockAnchor ||
    value === Capability.LiveStatus ||
    value === Capability.ReadPresetCatalog ||
    value === Capability.ProposePresetCatalog
  );
}

function isViewState(value: number): value is ViewState {
  return value >= ViewState.Idle && value <= ViewState.Completed && Number.isInteger(value);
}

function isJournalHealth(value: number): value is JournalHealth {
  return (
    value >= JournalHealth.Healthy && value <= JournalHealth.Unavailable && Number.isInteger(value)
  );
}

function isSessionOutcome(value: number): value is SessionOutcome {
  return value === SessionOutcome.Completed || value === SessionOutcome.Cancelled;
}

function isProtocolErrorCode(value: number): value is ProtocolErrorCode {
  return (
    value >= ProtocolErrorCode.MalformedEnvelope &&
    value <= ProtocolErrorCode.CatalogConflict &&
    Number.isInteger(value)
  );
}

function isCatalogResult(value: number): value is CatalogResult {
  return (
    value >= CatalogResult.Committed &&
    value <= CatalogResult.StorageFailed &&
    Number.isInteger(value)
  );
}

class CborWriter {
  private readonly bytes: number[] = [];

  public finish(): Uint8Array {
    if (this.bytes.length > MAX_LOGICAL_MESSAGE_BYTES) {
      throw new ProtocolEncodeError('messageTooLarge');
    }
    return Uint8Array.from(this.bytes);
  }

  public writeUint(value: number): void {
    validateSafeUint(value, 'uint');
    this.writeHead(0, value);
  }

  public writeBytes(value: Uint8Array): void {
    this.writeHead(2, value.length);
    this.putBytes(value);
  }

  public writeText(value: string): void {
    const encoded = encodeUtf8(value);
    this.writeHead(3, encoded.length);
    this.putBytes(encoded);
  }

  public writeArray(length: number): void {
    this.writeHead(4, length);
  }

  public writeMap(length: number): void {
    this.writeHead(5, length);
  }

  public writeBoolean(value: boolean): void {
    this.put(value ? 0xf5 : 0xf4);
  }

  private writeHead(major: number, value: number): void {
    validateSafeUint(value, 'cborValue');
    const prefix = major << 5;
    if (value <= 23) this.put(prefix | value);
    else if (value <= 0xff) {
      this.put(prefix | 0x18);
      this.put(value);
    } else if (value <= 0xffff) {
      this.put(prefix | 0x19);
      this.put((value >>> 8) & 0xff);
      this.put(value & 0xff);
    } else if (value <= MAX_UINT32) {
      this.put(prefix | 0x1a);
      this.put(Math.floor(value / 0x1_0000_00) & 0xff);
      this.put(Math.floor(value / 0x1_0000) & 0xff);
      this.put(Math.floor(value / 0x100) & 0xff);
      this.put(value & 0xff);
    } else {
      this.put(prefix | 0x1b);
      let remaining = BigInt(value);
      const encoded = new Uint8Array(8);
      for (let index = 7; index >= 0; index -= 1) {
        encoded[index] = Number(remaining & 0xffn);
        remaining >>= 8n;
      }
      this.putBytes(encoded);
    }
  }

  private put(value: number): void {
    this.bytes.push(value);
    if (this.bytes.length > MAX_LOGICAL_MESSAGE_BYTES) {
      throw new ProtocolEncodeError('messageTooLarge');
    }
  }

  private putBytes(value: Uint8Array): void {
    for (const byte of value) this.put(byte);
  }
}

class CborReader {
  private position = 0;

  public constructor(private readonly input: Uint8Array) {
    if (input.length === 0) throw new ProtocolDecodeError('empty');
    if (input.length > MAX_LOGICAL_MESSAGE_BYTES) throw new ProtocolDecodeError('messageTooLarge');
  }

  public finish(): void {
    if (this.position !== this.input.length) throw new ProtocolDecodeError('trailingData');
  }

  public readUint(): number {
    const [major, value] = this.readHead();
    if (major !== 0) throw new ProtocolDecodeError('unexpectedType');
    return value;
  }

  public readUint8(field: number): number {
    const value = this.readUint();
    if (value > MAX_UINT8) throw new ProtocolDecodeError('invalidValue', field);
    return value;
  }

  public readUint32(field: number): number {
    const value = this.readUint();
    if (value > MAX_UINT32) throw new ProtocolDecodeError('invalidValue', field);
    return value;
  }

  public readMapLength(): number {
    const [major, value] = this.readHead();
    if (major !== 5) throw new ProtocolDecodeError('unexpectedType');
    if (value > MAX_MAP_ENTRIES) throw new ProtocolDecodeError('limitExceeded');
    return value;
  }

  public readArrayLength(): number {
    const [major, value] = this.readHead();
    if (major !== 4) throw new ProtocolDecodeError('unexpectedType');
    if (value > MAX_ARRAY_ITEMS) throw new ProtocolDecodeError('limitExceeded');
    return value;
  }

  public readEmptyMap(): void {
    if (this.readMapLength() !== 0) throw new ProtocolDecodeError('invalidValue', 4);
  }

  public readMapKey(previous: number | undefined): number {
    const key = this.readUint();
    if (previous !== undefined && key <= previous) {
      throw new ProtocolDecodeError('duplicateOrUnorderedKey');
    }
    return key;
  }

  public readBytesExact(length: number, field: number): Uint8Array {
    const [major, actualLength] = this.readHead();
    if (major !== 2) throw new ProtocolDecodeError('unexpectedType');
    if (actualLength !== length) throw new ProtocolDecodeError('invalidValue', field);
    return this.take(length).slice();
  }

  public readText(maximumBytes: number, field: number): string {
    const [major, length] = this.readHead();
    if (major !== 3) throw new ProtocolDecodeError('unexpectedType');
    if (length > maximumBytes || length > MAX_TEXT_BYTES)
      throw new ProtocolDecodeError('limitExceeded');
    try {
      return decodeUtf8(this.take(length));
    } catch {
      throw new ProtocolDecodeError('invalidUtf8', field);
    }
  }

  public readBoolean(): boolean {
    const value = this.readByte();
    if (value === 0xf4) return false;
    if (value === 0xf5) return true;
    throw new ProtocolDecodeError('unexpectedType');
  }

  public skipMap(depth: number): void {
    const length = this.readMapLength();
    let previousKey: number | undefined;
    for (let index = 0; index < length; index += 1) {
      previousKey = this.readMapKey(previousKey);
      this.skipValue(depth + 1);
    }
  }

  public skipValue(depth: number): void {
    if (depth > MAX_NESTING_DEPTH) throw new ProtocolDecodeError('limitExceeded');
    const initial = this.peekByte();
    const major = initial >> 5;
    if (major === 0) {
      this.readHead();
      return;
    }
    if (major === 2 || major === 3) {
      const [, length] = this.readHead();
      if (length > MAX_TEXT_BYTES) throw new ProtocolDecodeError('limitExceeded');
      const bytes = this.take(length);
      if (major === 3) {
        try {
          decodeUtf8(bytes);
        } catch {
          throw new ProtocolDecodeError('invalidUtf8');
        }
      }
      return;
    }
    if (major === 4) {
      const length = this.readArrayLength();
      for (let index = 0; index < length; index += 1) this.skipValue(depth + 1);
      return;
    }
    if (major === 5) {
      this.skipMap(depth);
      return;
    }
    if (major === 7) {
      const value = this.readByte();
      if (value === 0xf4 || value === 0xf5) return;
    }
    throw new ProtocolDecodeError('unsupportedType');
  }

  private readHead(): readonly [number, number] {
    const initial = this.readByte();
    const major = initial >> 5;
    const additional = initial & 0x1f;
    if (additional <= 23) return [major, additional];
    if (additional === 24) {
      const value = this.readByte();
      if (value < 24) throw new ProtocolDecodeError('nonCanonical');
      return [major, value];
    }
    if (additional === 25) {
      const bytes = this.take(2);
      const value = bytes[0]! * 0x100 + bytes[1]!;
      if (value <= 0xff) throw new ProtocolDecodeError('nonCanonical');
      return [major, value];
    }
    if (additional === 26) {
      const bytes = this.take(4);
      const value = bytes[0]! * 0x1_0000_00 + bytes[1]! * 0x1_0000 + bytes[2]! * 0x100 + bytes[3]!;
      if (value <= 0xffff) throw new ProtocolDecodeError('nonCanonical');
      return [major, value];
    }
    if (additional === 27) {
      const bytes = this.take(8);
      let value = 0n;
      for (const byte of bytes) value = (value << 8n) | BigInt(byte);
      if (value <= BigInt(MAX_UINT32)) throw new ProtocolDecodeError('nonCanonical');
      if (value > BigInt(MAX_SAFE_PROTOCOL_INTEGER)) throw new ProtocolDecodeError('limitExceeded');
      return [major, Number(value)];
    }
    throw new ProtocolDecodeError('unsupportedType');
  }

  private peekByte(): number {
    const value = this.input[this.position];
    if (value === undefined) throw new ProtocolDecodeError('truncated');
    return value;
  }

  private readByte(): number {
    const value = this.peekByte();
    this.position += 1;
    return value;
  }

  private take(length: number): Uint8Array {
    const end = this.position + length;
    if (!Number.isSafeInteger(end) || end > this.input.length)
      throw new ProtocolDecodeError('truncated');
    const value = this.input.subarray(this.position, end);
    this.position = end;
    return value;
  }
}

function encodeUtf8(value: string): Uint8Array {
  const bytes: number[] = [];
  for (let index = 0; index < value.length; index += 1) {
    const first = value.charCodeAt(index);
    let codePoint = first;
    if (first >= 0xd800 && first <= 0xdbff) {
      const second = value.charCodeAt(index + 1);
      if (second < 0xdc00 || second > 0xdfff) invalid('utf8');
      codePoint = 0x1_0000 + ((first - 0xd800) << 10) + (second - 0xdc00);
      index += 1;
    } else if (first >= 0xdc00 && first <= 0xdfff) invalid('utf8');

    if (codePoint <= 0x7f) bytes.push(codePoint);
    else if (codePoint <= 0x7ff) bytes.push(0xc0 | (codePoint >> 6), 0x80 | (codePoint & 0x3f));
    else if (codePoint <= 0xffff) {
      bytes.push(
        0xe0 | (codePoint >> 12),
        0x80 | ((codePoint >> 6) & 0x3f),
        0x80 | (codePoint & 0x3f),
      );
    } else {
      bytes.push(
        0xf0 | (codePoint >> 18),
        0x80 | ((codePoint >> 12) & 0x3f),
        0x80 | ((codePoint >> 6) & 0x3f),
        0x80 | (codePoint & 0x3f),
      );
    }
  }
  return Uint8Array.from(bytes);
}

function decodeUtf8(bytes: Uint8Array): string {
  const codePoints: number[] = [];
  for (let index = 0; index < bytes.length;) {
    const first = bytes[index]!;
    if (first <= 0x7f) {
      codePoints.push(first);
      index += 1;
      continue;
    }
    let needed: number;
    let codePoint: number;
    let minimum: number;
    if (first >= 0xc2 && first <= 0xdf) {
      needed = 1;
      codePoint = first & 0x1f;
      minimum = 0x80;
    } else if (first >= 0xe0 && first <= 0xef) {
      needed = 2;
      codePoint = first & 0x0f;
      minimum = 0x800;
    } else if (first >= 0xf0 && first <= 0xf4) {
      needed = 3;
      codePoint = first & 0x07;
      minimum = 0x1_0000;
    } else throw new Error('invalid UTF-8');

    if (index + needed >= bytes.length) throw new Error('truncated UTF-8');
    for (let offset = 1; offset <= needed; offset += 1) {
      const continuation = bytes[index + offset]!;
      if ((continuation & 0xc0) !== 0x80) throw new Error('invalid UTF-8 continuation');
      codePoint = (codePoint << 6) | (continuation & 0x3f);
    }
    if (
      codePoint < minimum ||
      codePoint > 0x10_ffff ||
      (codePoint >= 0xd800 && codePoint <= 0xdfff)
    ) {
      throw new Error('non-canonical UTF-8');
    }
    codePoints.push(codePoint);
    index += needed + 1;
  }
  return String.fromCodePoint(...codePoints);
}
