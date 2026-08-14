use focus_protocol::{
    BLE_FRAME_HEADER_BYTES, BLE_REASSEMBLY_TIMEOUT_MS, FragmentError, Fragmenter, Reassembler,
    ReassemblyError, ReassemblyStatus, crc32_iso_hdlc, next_transfer_id,
};

fn frames(message: &[u8], transfer_id: u16, maximum_frame_bytes: usize) -> Vec<Vec<u8>> {
    let mut fragmenter =
        Fragmenter::new(message, transfer_id, maximum_frame_bytes).expect("fragmenter must start");
    let mut result = Vec::new();
    let mut output = vec![0; maximum_frame_bytes];
    while let Some(length) = fragmenter
        .next_frame(&mut output)
        .expect("frame must encode")
    {
        result.push(output[..length].to_vec());
    }
    result
}

#[test]
fn crc_matches_the_registry_check_value() {
    assert_eq!(crc32_iso_hdlc(b"123456789"), 0xcbf4_3926);
}

#[test]
fn one_and_many_frame_messages_round_trip() {
    for (message, maximum_frame_bytes) in [
        (b"hello".as_slice(), 64),
        (
            b"a longer logical message".as_slice(),
            BLE_FRAME_HEADER_BYTES + 5,
        ),
    ] {
        let frames = frames(message, 7, maximum_frame_bytes);
        let mut reassembler = Reassembler::new();
        for (index, frame) in frames.iter().enumerate() {
            let status = reassembler
                .accept_frame(frame, u64::try_from(index).expect("index must fit") * 10)
                .expect("frame must reassemble");
            if index + 1 == frames.len() {
                assert_eq!(
                    status,
                    ReassemblyStatus::Complete {
                        message_length: message.len()
                    }
                );
            } else {
                assert_eq!(status, ReassemblyStatus::InProgress);
            }
        }
        assert_eq!(reassembler.completed_message(), Some(message));
    }
}

#[test]
fn frame_header_is_big_endian_and_correlated() {
    let frames = frames(b"abc", 0x1234, 64);
    assert_eq!(frames.len(), 1);
    let frame = &frames[0];
    assert_eq!(&frame[2..4], &[0x12, 0x34]);
    assert_eq!(&frame[4..6], &[0x00, 0x03]);
    assert_eq!(&frame[6..8], &[0x00, 0x00]);
    assert_eq!(&frame[8..12], &crc32_iso_hdlc(b"abc").to_be_bytes());
}

#[test]
fn timeout_and_bad_order_reset_the_single_active_transfer() {
    let source = frames(b"abcdefghij", 9, BLE_FRAME_HEADER_BYTES + 4);
    let mut reassembler = Reassembler::new();
    assert_eq!(
        reassembler.accept_frame(&source[0], 10),
        Ok(ReassemblyStatus::InProgress)
    );
    assert_eq!(
        reassembler.accept_frame(&source[1], 10 + BLE_REASSEMBLY_TIMEOUT_MS + 1),
        Err(ReassemblyError::TimedOut)
    );
    assert_eq!(reassembler.completed_message(), None);

    assert_eq!(
        reassembler.accept_frame(&source[0], 20_000),
        Ok(ReassemblyStatus::InProgress)
    );
    assert_eq!(
        reassembler.accept_frame(&source[2], 20_010),
        Err(ReassemblyError::NonContiguousOffset)
    );
}

#[test]
fn checksum_failure_does_not_expose_partial_bytes() {
    let mut source = frames(b"checksum", 11, 64);
    let frame = source.first_mut().expect("single frame must exist");
    *frame.last_mut().expect("payload must exist") ^= 0x01;
    let mut reassembler = Reassembler::new();
    assert_eq!(
        reassembler.accept_frame(frame, 0),
        Err(ReassemblyError::ChecksumMismatch)
    );
    assert_eq!(reassembler.completed_message(), None);
}

#[test]
fn fragmentation_inputs_and_transfer_increment_are_bounded() {
    assert!(matches!(
        Fragmenter::new(&[], 1, 20),
        Err(FragmentError::EmptyMessage)
    ));
    assert!(matches!(
        Fragmenter::new(b"x", 0, 20),
        Err(FragmentError::InvalidTransferId)
    ));
    assert!(matches!(
        Fragmenter::new(b"x", 1, BLE_FRAME_HEADER_BYTES),
        Err(FragmentError::FrameCapacityTooSmall)
    ));
    assert_eq!(next_transfer_id(1), 2);
    assert_eq!(next_transfer_id(u16::MAX), 1);
}
