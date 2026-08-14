use crate::MAX_LOGICAL_MESSAGE_BYTES;

pub const BLE_FRAME_VERSION: u8 = 1;
pub const BLE_FRAME_HEADER_BYTES: usize = 12;
pub const BLE_REASSEMBLY_TIMEOUT_MS: u64 = 3_000;
pub const BLE_START_FLAG: u8 = 0x01;
pub const BLE_END_FLAG: u8 = 0x02;

const BLE_KNOWN_FLAGS: u8 = BLE_START_FLAG | BLE_END_FLAG;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FragmentError {
    EmptyMessage,
    MessageTooLarge,
    InvalidTransferId,
    FrameCapacityTooSmall,
    OutputTooSmall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReassemblyError {
    FrameTooShort,
    UnsupportedFrameVersion,
    ReservedFlags,
    InvalidTransferId,
    InvalidTotalLength,
    EmptyPayload,
    InvalidStartFlag,
    InvalidEndFlag,
    OffsetOverflow,
    UnexpectedFragment,
    DuplicateStart,
    MetadataMismatch,
    NonContiguousOffset,
    TimedOut,
    ChecksumMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameHeader {
    pub flags: u8,
    pub transfer_id: u16,
    pub total_length: u16,
    pub fragment_offset: u16,
    pub message_crc32: u32,
}

impl FrameHeader {
    #[must_use]
    pub const fn is_start(self) -> bool {
        self.flags & BLE_START_FLAG != 0
    }

    #[must_use]
    pub const fn is_end(self) -> bool {
        self.flags & BLE_END_FLAG != 0
    }
}

pub struct Fragmenter<'a> {
    message: &'a [u8],
    transfer_id: u16,
    maximum_frame_bytes: usize,
    total_length: u16,
    offset: usize,
    checksum: u32,
}

impl<'a> Fragmenter<'a> {
    /// Creates a bounded fragment stream for one complete logical message.
    ///
    /// # Errors
    ///
    /// Returns [`FragmentError`] when the message, transfer identifier, or
    /// negotiated characteristic capacity violates the protocol registry.
    pub fn new(
        message: &'a [u8],
        transfer_id: u16,
        maximum_frame_bytes: usize,
    ) -> Result<Self, FragmentError> {
        if message.is_empty() {
            return Err(FragmentError::EmptyMessage);
        }
        if message.len() > MAX_LOGICAL_MESSAGE_BYTES {
            return Err(FragmentError::MessageTooLarge);
        }
        let total_length =
            u16::try_from(message.len()).map_err(|_| FragmentError::MessageTooLarge)?;
        if transfer_id == 0 {
            return Err(FragmentError::InvalidTransferId);
        }
        if maximum_frame_bytes <= BLE_FRAME_HEADER_BYTES {
            return Err(FragmentError::FrameCapacityTooSmall);
        }
        Ok(Self {
            message,
            transfer_id,
            maximum_frame_bytes,
            total_length,
            offset: 0,
            checksum: crc32_iso_hdlc(message),
        })
    }

    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.offset == self.message.len()
    }

    /// Encodes the next contiguous BLE frame into `output`.
    ///
    /// # Errors
    ///
    /// Returns [`FragmentError::OutputTooSmall`] when `output` cannot hold the
    /// next frame up to the negotiated characteristic value size.
    pub fn next_frame(&mut self, output: &mut [u8]) -> Result<Option<usize>, FragmentError> {
        encode_next_frame(
            self.message,
            self.transfer_id,
            self.maximum_frame_bytes,
            self.total_length,
            self.checksum,
            &mut self.offset,
            output,
        )
    }
}

/// An owned bounded fragment stream suitable for a non-blocking firmware
/// outbox that emits one notification per event-loop iteration.
pub struct OwnedFragmenter {
    message: [u8; MAX_LOGICAL_MESSAGE_BYTES],
    message_length: usize,
    transfer_id: u16,
    maximum_frame_bytes: usize,
    total_length: u16,
    offset: usize,
    checksum: u32,
}

impl OwnedFragmenter {
    /// Copies one bounded logical response into an owned transfer.
    ///
    /// # Errors
    ///
    /// Applies the same transfer identifier, message, and frame-capacity
    /// bounds as [`Fragmenter::new`].
    pub fn new(
        message: &[u8],
        transfer_id: u16,
        maximum_frame_bytes: usize,
    ) -> Result<Self, FragmentError> {
        let fragmenter = Fragmenter::new(message, transfer_id, maximum_frame_bytes)?;
        let mut owned = Self {
            message: [0; MAX_LOGICAL_MESSAGE_BYTES],
            message_length: message.len(),
            transfer_id,
            maximum_frame_bytes,
            total_length: fragmenter.total_length,
            offset: 0,
            checksum: fragmenter.checksum,
        };
        owned.message[..message.len()].copy_from_slice(message);
        Ok(owned)
    }

    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.offset == self.message_length
    }

    /// Encodes the next contiguous frame while retaining owned transfer state.
    ///
    /// # Errors
    ///
    /// Returns [`FragmentError::OutputTooSmall`] when `output` cannot hold the
    /// next frame up to the negotiated characteristic value size.
    pub fn next_frame(&mut self, output: &mut [u8]) -> Result<Option<usize>, FragmentError> {
        encode_next_frame(
            &self.message[..self.message_length],
            self.transfer_id,
            self.maximum_frame_bytes,
            self.total_length,
            self.checksum,
            &mut self.offset,
            output,
        )
    }
}

fn encode_next_frame(
    message: &[u8],
    transfer_id: u16,
    maximum_frame_bytes: usize,
    total_length: u16,
    checksum: u32,
    offset: &mut usize,
    output: &mut [u8],
) -> Result<Option<usize>, FragmentError> {
    if *offset == message.len() {
        return Ok(None);
    }
    let payload_capacity = maximum_frame_bytes - BLE_FRAME_HEADER_BYTES;
    let payload_length = payload_capacity.min(message.len() - *offset);
    let frame_length = BLE_FRAME_HEADER_BYTES + payload_length;
    if output.len() < frame_length {
        return Err(FragmentError::OutputTooSmall);
    }

    let end_offset = *offset + payload_length;
    let mut flags = 0;
    if *offset == 0 {
        flags |= BLE_START_FLAG;
    }
    if end_offset == message.len() {
        flags |= BLE_END_FLAG;
    }
    output[0] = BLE_FRAME_VERSION;
    output[1] = flags;
    output[2..4].copy_from_slice(&transfer_id.to_be_bytes());
    output[4..6].copy_from_slice(&total_length.to_be_bytes());
    let fragment_offset = u16::try_from(*offset).map_err(|_| FragmentError::MessageTooLarge)?;
    output[6..8].copy_from_slice(&fragment_offset.to_be_bytes());
    output[8..12].copy_from_slice(&checksum.to_be_bytes());
    output[12..frame_length].copy_from_slice(&message[*offset..end_offset]);
    *offset = end_offset;
    Ok(Some(frame_length))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReassemblyStatus {
    InProgress,
    Complete { message_length: usize },
}

pub struct Reassembler {
    buffer: [u8; MAX_LOGICAL_MESSAGE_BYTES],
    active: Option<ActiveTransfer>,
    completed_length: Option<usize>,
}

#[derive(Clone, Copy)]
struct ActiveTransfer {
    transfer_id: u16,
    total_length: usize,
    next_offset: usize,
    checksum: u32,
    last_accepted_at_ms: u64,
}

impl Reassembler {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: [0; MAX_LOGICAL_MESSAGE_BYTES],
            active: None,
            completed_length: None,
        }
    }

    pub fn reset(&mut self) {
        self.active = None;
        self.completed_length = None;
    }

    /// Drops an incomplete transfer after the registry timeout even when no
    /// further fragment arrives.
    #[must_use]
    pub fn expire(&mut self, now_ms: u64) -> bool {
        let timed_out = self.active.is_some_and(|active| {
            now_ms.saturating_sub(active.last_accepted_at_ms) > BLE_REASSEMBLY_TIMEOUT_MS
        });
        if timed_out {
            self.reset();
        }
        timed_out
    }

    #[must_use]
    pub fn completed_message(&self) -> Option<&[u8]> {
        self.completed_length.map(|length| &self.buffer[..length])
    }

    /// Accepts one frame while keeping at most one transfer in progress.
    ///
    /// # Errors
    ///
    /// Invalid order, metadata, bounds, timeout, or checksum resets the active
    /// transfer before returning a [`ReassemblyError`].
    pub fn accept_frame(
        &mut self,
        frame: &[u8],
        now_ms: u64,
    ) -> Result<ReassemblyStatus, ReassemblyError> {
        self.completed_length = None;
        let (header, payload) = match parse_frame(frame) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.reset();
                return Err(error);
            }
        };

        if let Some(active) = self.active
            && now_ms.saturating_sub(active.last_accepted_at_ms) > BLE_REASSEMBLY_TIMEOUT_MS
        {
            self.reset();
            if !header.is_start() {
                return Err(ReassemblyError::TimedOut);
            }
        }

        if header.is_start() {
            if let Some(active) = self.active {
                let duplicate = active.transfer_id == header.transfer_id;
                self.reset();
                if duplicate {
                    return Err(ReassemblyError::DuplicateStart);
                }
            }
            self.active = Some(ActiveTransfer {
                transfer_id: header.transfer_id,
                total_length: usize::from(header.total_length),
                next_offset: 0,
                checksum: header.message_crc32,
                last_accepted_at_ms: now_ms,
            });
        }

        let Some(mut active) = self.active else {
            return Err(ReassemblyError::UnexpectedFragment);
        };
        if active.transfer_id != header.transfer_id
            || active.total_length != usize::from(header.total_length)
            || active.checksum != header.message_crc32
        {
            self.reset();
            return Err(ReassemblyError::MetadataMismatch);
        }
        if active.next_offset != usize::from(header.fragment_offset) {
            self.reset();
            return Err(ReassemblyError::NonContiguousOffset);
        }

        let end = active
            .next_offset
            .checked_add(payload.len())
            .ok_or(ReassemblyError::OffsetOverflow)?;
        if end > active.total_length {
            self.reset();
            return Err(ReassemblyError::OffsetOverflow);
        }
        self.buffer[active.next_offset..end].copy_from_slice(payload);
        active.next_offset = end;
        active.last_accepted_at_ms = now_ms;

        if header.is_end() {
            if crc32_iso_hdlc(&self.buffer[..end]) != active.checksum {
                self.reset();
                return Err(ReassemblyError::ChecksumMismatch);
            }
            self.active = None;
            self.completed_length = Some(end);
            Ok(ReassemblyStatus::Complete {
                message_length: end,
            })
        } else {
            self.active = Some(active);
            Ok(ReassemblyStatus::InProgress)
        }
    }
}

impl Default for Reassembler {
    fn default() -> Self {
        Self::new()
    }
}

/// Parses and validates the frame-local header invariants.
///
/// # Errors
///
/// Returns a [`ReassemblyError`] for a malformed or registry-incompatible frame.
pub fn parse_frame(frame: &[u8]) -> Result<(FrameHeader, &[u8]), ReassemblyError> {
    if frame.len() < BLE_FRAME_HEADER_BYTES + 1 {
        return Err(ReassemblyError::FrameTooShort);
    }
    if frame[0] != BLE_FRAME_VERSION {
        return Err(ReassemblyError::UnsupportedFrameVersion);
    }
    let flags = frame[1];
    if flags & !BLE_KNOWN_FLAGS != 0 {
        return Err(ReassemblyError::ReservedFlags);
    }
    let transfer_id = u16::from_be_bytes([frame[2], frame[3]]);
    if transfer_id == 0 {
        return Err(ReassemblyError::InvalidTransferId);
    }
    let total_length = u16::from_be_bytes([frame[4], frame[5]]);
    if total_length == 0 || usize::from(total_length) > MAX_LOGICAL_MESSAGE_BYTES {
        return Err(ReassemblyError::InvalidTotalLength);
    }
    let fragment_offset = u16::from_be_bytes([frame[6], frame[7]]);
    let checksum = u32::from_be_bytes([frame[8], frame[9], frame[10], frame[11]]);
    let payload = &frame[BLE_FRAME_HEADER_BYTES..];
    if payload.is_empty() {
        return Err(ReassemblyError::EmptyPayload);
    }
    let end = usize::from(fragment_offset)
        .checked_add(payload.len())
        .ok_or(ReassemblyError::OffsetOverflow)?;
    if end > usize::from(total_length) {
        return Err(ReassemblyError::OffsetOverflow);
    }
    let is_start = flags & BLE_START_FLAG != 0;
    if is_start != (fragment_offset == 0) {
        return Err(ReassemblyError::InvalidStartFlag);
    }
    let is_end = flags & BLE_END_FLAG != 0;
    if is_end != (end == usize::from(total_length)) {
        return Err(ReassemblyError::InvalidEndFlag);
    }
    Ok((
        FrameHeader {
            flags,
            transfer_id,
            total_length,
            fragment_offset,
            message_crc32: checksum,
        },
        payload,
    ))
}

#[must_use]
pub const fn next_transfer_id(current: u16) -> u16 {
    let next = current.wrapping_add(1);
    if next == 0 { 1 } else { next }
}

#[must_use]
pub fn crc32_iso_hdlc(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    crc ^ 0xffff_ffff
}
