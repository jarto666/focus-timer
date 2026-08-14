import { MAX_LOGICAL_MESSAGE_BYTES } from './model';

export const BLE_FRAME_VERSION = 1;
export const BLE_FRAME_HEADER_BYTES = 12;
export const BLE_REASSEMBLY_TIMEOUT_MS = 3_000;
export const BLE_START_FLAG = 0x01;
export const BLE_END_FLAG = 0x02;

const BLE_KNOWN_FLAGS = BLE_START_FLAG | BLE_END_FLAG;
const MAX_TRANSFER_ID = 0xffff;

export type FragmentErrorCode =
  'emptyMessage' | 'messageTooLarge' | 'invalidTransferId' | 'frameCapacityTooSmall';

export class BleFragmentError extends Error {
  public constructor(public readonly code: FragmentErrorCode) {
    super(code);
    this.name = 'BleFragmentError';
  }
}

export type ReassemblyErrorCode =
  | 'frameTooShort'
  | 'unsupportedFrameVersion'
  | 'reservedFlags'
  | 'invalidTransferId'
  | 'invalidTotalLength'
  | 'emptyPayload'
  | 'invalidStartFlag'
  | 'invalidEndFlag'
  | 'offsetOverflow'
  | 'unexpectedFragment'
  | 'duplicateStart'
  | 'metadataMismatch'
  | 'nonContiguousOffset'
  | 'timedOut'
  | 'checksumMismatch';

export class BleReassemblyError extends Error {
  public constructor(public readonly code: ReassemblyErrorCode) {
    super(code);
    this.name = 'BleReassemblyError';
  }
}

export interface BleFrameHeader {
  readonly flags: number;
  readonly transferId: number;
  readonly totalLength: number;
  readonly fragmentOffset: number;
  readonly messageCrc32: number;
}

export interface ParsedBleFrame {
  readonly header: BleFrameHeader;
  readonly payload: Uint8Array;
}

export class BleFragmenter {
  private offset = 0;
  private readonly checksum: number;

  public constructor(
    private readonly message: Uint8Array,
    private readonly transferId: number,
    private readonly maximumFrameBytes: number,
  ) {
    if (message.length === 0) throw new BleFragmentError('emptyMessage');
    if (message.length > MAX_LOGICAL_MESSAGE_BYTES) throw new BleFragmentError('messageTooLarge');
    validateTransferId(transferId);
    if (!Number.isSafeInteger(maximumFrameBytes) || maximumFrameBytes <= BLE_FRAME_HEADER_BYTES) {
      throw new BleFragmentError('frameCapacityTooSmall');
    }
    this.checksum = crc32IsoHdlc(message);
  }

  public get isComplete(): boolean {
    return this.offset === this.message.length;
  }

  public nextFrame(): Uint8Array | undefined {
    if (this.isComplete) return undefined;
    const payloadCapacity = this.maximumFrameBytes - BLE_FRAME_HEADER_BYTES;
    const payloadLength = Math.min(payloadCapacity, this.message.length - this.offset);
    const endOffset = this.offset + payloadLength;
    let flags = 0;
    if (this.offset === 0) flags |= BLE_START_FLAG;
    if (endOffset === this.message.length) flags |= BLE_END_FLAG;

    const frame = new Uint8Array(BLE_FRAME_HEADER_BYTES + payloadLength);
    frame[0] = BLE_FRAME_VERSION;
    frame[1] = flags;
    writeUint16(frame, 2, this.transferId);
    writeUint16(frame, 4, this.message.length);
    writeUint16(frame, 6, this.offset);
    writeUint32(frame, 8, this.checksum);
    frame.set(this.message.subarray(this.offset, endOffset), BLE_FRAME_HEADER_BYTES);
    this.offset = endOffset;
    return frame;
  }
}

export type ReassemblyResult =
  { readonly status: 'inProgress' } | { readonly status: 'complete'; readonly message: Uint8Array };

interface ActiveTransfer {
  readonly transferId: number;
  readonly totalLength: number;
  nextOffset: number;
  readonly checksum: number;
  lastAcceptedAtMs: number;
}

export class BleReassembler {
  private readonly buffer = new Uint8Array(MAX_LOGICAL_MESSAGE_BYTES);
  private active: ActiveTransfer | undefined;

  public reset(): void {
    this.active = undefined;
  }

  public acceptFrame(frame: Uint8Array, nowMs: number): ReassemblyResult {
    if (!Number.isSafeInteger(nowMs) || nowMs < 0) {
      throw new BleReassemblyError('timedOut');
    }
    let parsed: ParsedBleFrame;
    try {
      parsed = parseBleFrame(frame);
    } catch (error) {
      this.reset();
      throw error;
    }
    const { header, payload } = parsed;

    if (
      this.active !== undefined &&
      Math.max(0, nowMs - this.active.lastAcceptedAtMs) > BLE_REASSEMBLY_TIMEOUT_MS
    ) {
      this.reset();
      if (!isStart(header)) throw new BleReassemblyError('timedOut');
    }

    if (isStart(header)) {
      if (this.active !== undefined) {
        const duplicate = this.active.transferId === header.transferId;
        this.reset();
        if (duplicate) throw new BleReassemblyError('duplicateStart');
      }
      this.active = {
        transferId: header.transferId,
        totalLength: header.totalLength,
        nextOffset: 0,
        checksum: header.messageCrc32,
        lastAcceptedAtMs: nowMs,
      };
    }

    const active = this.active;
    if (active === undefined) throw new BleReassemblyError('unexpectedFragment');
    if (
      active.transferId !== header.transferId ||
      active.totalLength !== header.totalLength ||
      active.checksum !== header.messageCrc32
    ) {
      this.reset();
      throw new BleReassemblyError('metadataMismatch');
    }
    if (active.nextOffset !== header.fragmentOffset) {
      this.reset();
      throw new BleReassemblyError('nonContiguousOffset');
    }

    const end = active.nextOffset + payload.length;
    if (!Number.isSafeInteger(end) || end > active.totalLength) {
      this.reset();
      throw new BleReassemblyError('offsetOverflow');
    }
    this.buffer.set(payload, active.nextOffset);
    active.nextOffset = end;
    active.lastAcceptedAtMs = nowMs;

    if (!isEnd(header)) return { status: 'inProgress' };
    if (crc32IsoHdlc(this.buffer.subarray(0, end)) !== active.checksum) {
      this.reset();
      throw new BleReassemblyError('checksumMismatch');
    }
    this.reset();
    return { status: 'complete', message: this.buffer.slice(0, end) };
  }
}

export function parseBleFrame(frame: Uint8Array): ParsedBleFrame {
  if (frame.length < BLE_FRAME_HEADER_BYTES + 1) throw new BleReassemblyError('frameTooShort');
  if (frame[0] !== BLE_FRAME_VERSION) throw new BleReassemblyError('unsupportedFrameVersion');
  const flags = frame[1]!;
  if ((flags & ~BLE_KNOWN_FLAGS) !== 0) throw new BleReassemblyError('reservedFlags');
  const transferId = readUint16(frame, 2);
  if (transferId === 0) throw new BleReassemblyError('invalidTransferId');
  const totalLength = readUint16(frame, 4);
  if (totalLength === 0 || totalLength > MAX_LOGICAL_MESSAGE_BYTES) {
    throw new BleReassemblyError('invalidTotalLength');
  }
  const fragmentOffset = readUint16(frame, 6);
  const messageCrc32 = readUint32(frame, 8);
  const payload = frame.subarray(BLE_FRAME_HEADER_BYTES);
  if (payload.length === 0) throw new BleReassemblyError('emptyPayload');
  const end = fragmentOffset + payload.length;
  if (!Number.isSafeInteger(end) || end > totalLength)
    throw new BleReassemblyError('offsetOverflow');

  const header: BleFrameHeader = {
    flags,
    transferId,
    totalLength,
    fragmentOffset,
    messageCrc32,
  };
  if (isStart(header) !== (fragmentOffset === 0)) throw new BleReassemblyError('invalidStartFlag');
  if (isEnd(header) !== (end === totalLength)) throw new BleReassemblyError('invalidEndFlag');
  return { header, payload };
}

export function nextTransferId(current: number): number {
  if (!Number.isInteger(current) || current < 0 || current > MAX_TRANSFER_ID) {
    throw new BleFragmentError('invalidTransferId');
  }
  return current === MAX_TRANSFER_ID ? 1 : current + 1;
}

export function crc32IsoHdlc(bytes: Uint8Array): number {
  let crc = 0xffff_ffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc & 1) !== 0 ? (crc >>> 1) ^ 0xedb8_8320 : crc >>> 1;
    }
  }
  return (crc ^ 0xffff_ffff) >>> 0;
}

function isStart(header: BleFrameHeader): boolean {
  return (header.flags & BLE_START_FLAG) !== 0;
}

function isEnd(header: BleFrameHeader): boolean {
  return (header.flags & BLE_END_FLAG) !== 0;
}

function validateTransferId(value: number): void {
  if (!Number.isInteger(value) || value <= 0 || value > MAX_TRANSFER_ID) {
    throw new BleFragmentError('invalidTransferId');
  }
}

function readUint16(bytes: Uint8Array, offset: number): number {
  return bytes[offset]! * 0x100 + bytes[offset + 1]!;
}

function readUint32(bytes: Uint8Array, offset: number): number {
  return (
    bytes[offset]! * 0x1_0000_00 +
    bytes[offset + 1]! * 0x1_0000 +
    bytes[offset + 2]! * 0x100 +
    bytes[offset + 3]!
  );
}

function writeUint16(bytes: Uint8Array, offset: number, value: number): void {
  bytes[offset] = Math.floor(value / 0x100) & 0xff;
  bytes[offset + 1] = value & 0xff;
}

function writeUint32(bytes: Uint8Array, offset: number, value: number): void {
  bytes[offset] = Math.floor(value / 0x1_0000_00) & 0xff;
  bytes[offset + 1] = Math.floor(value / 0x1_0000) & 0xff;
  bytes[offset + 2] = Math.floor(value / 0x100) & 0xff;
  bytes[offset + 3] = value & 0xff;
}
