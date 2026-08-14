use focus_protocol::{
    BLE_FRAME_HEADER_BYTES, DecodeError, ErrorCode, ErrorResponse, Fragmenter,
    MAX_LOGICAL_MESSAGE_BYTES, ProtocolVersion, Reassembler, ReassemblyError, ReassemblyStatus,
    Request, RequestEnvelope, Response, ResponseEnvelope, decode_request, decode_response,
    encode_request, encode_response,
};

const HELLO: &[u8] = &[
    0xa5, 0x00, 0x01, 0x01, 0x00, 0x02, 0x01, 0x03, 0x01, 0x04, 0xa0,
];

fn frames(message: &[u8], transfer_id: u16) -> Vec<Vec<u8>> {
    let maximum_frame_bytes = BLE_FRAME_HEADER_BYTES + 4;
    let mut fragmenter =
        Fragmenter::new(message, transfer_id, maximum_frame_bytes).expect("message must fragment");
    let mut output = vec![0; maximum_frame_bytes];
    let mut result = Vec::new();
    while let Some(length) = fragmenter
        .next_frame(&mut output)
        .expect("frame must encode")
    {
        result.push(output[..length].to_vec());
    }
    result
}

fn reassemble_all(reassembler: &mut Reassembler, source: &[Vec<u8>], start_ms: u64) {
    for (index, frame) in source.iter().enumerate() {
        let status = reassembler
            .accept_frame(
                frame,
                start_ms + u64::try_from(index).expect("index must fit"),
            )
            .expect("valid retry must reassemble");
        if index + 1 == source.len() {
            assert!(matches!(status, ReassemblyStatus::Complete { .. }));
        }
    }
}

#[test]
fn every_truncated_prefix_and_oversized_input_is_rejected_without_panic() {
    for length in 0..HELLO.len() {
        assert!(decode_request(&HELLO[..length]).is_err());
    }
    assert_eq!(
        decode_request(HELLO),
        Ok(RequestEnvelope {
            version: ProtocolVersion::CURRENT,
            request_id: 1,
            request: Request::Hello,
        })
    );
    assert_eq!(
        decode_request(&[0; MAX_LOGICAL_MESSAGE_BYTES + 1]),
        Err(DecodeError::MessageTooLarge)
    );
}

#[test]
fn duplicate_keys_and_unsupported_response_kinds_are_explicit_errors() {
    let duplicate_key = [
        0xa5, 0x00, 0x01, 0x00, 0x00, 0x02, 0x01, 0x03, 0x01, 0x04, 0xa0,
    ];
    assert_eq!(
        decode_request(&duplicate_key),
        Err(DecodeError::DuplicateOrUnorderedKey)
    );

    let unsupported_response = [
        0xa5, 0x00, 0x01, 0x01, 0x00, 0x02, 0x01, 0x03, 0x09, 0x04, 0xa0,
    ];
    assert_eq!(
        decode_response(&unsupported_response),
        Err(DecodeError::UnsupportedMessage(9))
    );
}

#[test]
fn unknown_requests_and_incompatible_versions_remain_bounded_handler_inputs() {
    let unknown_request = [
        0xa5, 0x00, 0x01, 0x01, 0x00, 0x02, 0x01, 0x03, 0x09, 0x04, 0xa0,
    ];
    assert_eq!(
        decode_request(&unknown_request),
        Ok(RequestEnvelope {
            version: ProtocolVersion::CURRENT,
            request_id: 1,
            request: Request::Unknown { message_kind: 9 },
        })
    );

    let incompatible_request = [
        0xa5, 0x00, 0x02, 0x01, 0x00, 0x02, 0x07, 0x03, 0x01, 0x04, 0xa0,
    ];
    let decoded =
        decode_request(&incompatible_request).expect("handler must see incompatible major");
    assert_eq!(decoded.version.major, 2);
    let response = ResponseEnvelope {
        version: ProtocolVersion::CURRENT,
        request_id: decoded.request_id,
        response: Response::Error(ErrorResponse {
            code: ErrorCode::UnsupportedProtocolVersion,
            failed_message_kind: Some(1),
            field_id: None,
            supported_version: Some(ProtocolVersion::CURRENT),
        }),
    };
    let mut output = [0; MAX_LOGICAL_MESSAGE_BYTES];
    let length = encode_response(&response, &mut output).expect("version error must encode");
    assert_eq!(decode_response(&output[..length]), Ok(response));
}

#[test]
fn out_of_order_and_checksum_failures_reset_for_a_clean_retry() {
    let message = b"a fragmented logical payload";
    let source = frames(message, 21);
    let mut reassembler = Reassembler::new();
    assert_eq!(
        reassembler.accept_frame(&source[0], 0),
        Ok(ReassemblyStatus::InProgress)
    );
    assert_eq!(
        reassembler.accept_frame(&source[2], 1),
        Err(ReassemblyError::NonContiguousOffset)
    );
    reassemble_all(&mut reassembler, &source, 10);
    assert_eq!(reassembler.completed_message(), Some(message.as_slice()));

    let mut single_fragmenter = Fragmenter::new(b"checksum", 22, 64).expect("message must fit");
    let mut output = [0; 64];
    let length = single_fragmenter
        .next_frame(&mut output)
        .expect("frame must encode")
        .expect("frame must exist");
    let valid = output[..length].to_vec();
    let mut corrupted = valid.clone();
    *corrupted.last_mut().expect("payload must exist") ^= 1;
    assert_eq!(
        reassembler.accept_frame(&corrupted, 100),
        Err(ReassemblyError::ChecksumMismatch)
    );
    assert!(matches!(
        reassembler.accept_frame(&valid, 101),
        Ok(ReassemblyStatus::Complete { .. })
    ));
    assert_eq!(
        reassembler.completed_message(),
        Some(b"checksum".as_slice())
    );
}

#[test]
fn malformed_and_oversized_frames_clear_the_active_transfer() {
    let source = frames(b"abcdefghij", 31);
    let mut reassembler = Reassembler::new();
    assert_eq!(
        reassembler.accept_frame(&source[0], 0),
        Ok(ReassemblyStatus::InProgress)
    );
    let mut reserved = source[1].clone();
    reserved[1] |= 0x80;
    assert_eq!(
        reassembler.accept_frame(&reserved, 1),
        Err(ReassemblyError::ReservedFlags)
    );
    assert_eq!(
        reassembler.accept_frame(&source[1], 2),
        Err(ReassemblyError::UnexpectedFragment)
    );

    let mut oversized = source[0].clone();
    oversized[4..6].copy_from_slice(&2049_u16.to_be_bytes());
    assert_eq!(
        reassembler.accept_frame(&oversized, 3),
        Err(ReassemblyError::InvalidTotalLength)
    );
}

#[test]
fn encoders_reject_oversized_output_models_without_partial_success() {
    let envelope = RequestEnvelope {
        version: ProtocolVersion::CURRENT,
        request_id: 1,
        request: Request::Hello,
    };
    assert!(encode_request(&envelope, &mut [0; 4]).is_err());
}
