use focus_sync::{ClockError, MAX_JOURNAL_SEQUENCE, VolatileClock};

#[test]
fn cold_boot_time_is_unknown() {
    let clock = VolatileClock::new();
    assert!(!clock.is_known());
    assert_eq!(clock.timestamp_ms(100).unwrap(), None);
}

#[test]
fn anchor_derives_utc_from_monotonic_elapsed_time() {
    let mut clock = VolatileClock::new();
    clock.set_anchor(1_700_000_000_000, 10_000).unwrap();

    assert!(clock.is_known());
    assert_eq!(clock.timestamp_ms(12_500).unwrap(), Some(1_700_000_002_500));
}

#[test]
fn backwards_and_overflowing_time_are_rejected() {
    let mut clock = VolatileClock::new();
    clock.set_anchor(50_000, 1_000).unwrap();
    assert_eq!(
        clock.timestamp_ms(999).unwrap_err(),
        ClockError::MonotonicMovedBackwards
    );
    assert_eq!(
        clock.set_anchor(50_001, 999).unwrap_err(),
        ClockError::MonotonicMovedBackwards
    );
    assert_eq!(
        clock.set_anchor(49_999, 2_000).unwrap_err(),
        ClockError::UtcMovedBackwards
    );

    let mut near_limit = VolatileClock::new();
    near_limit.set_anchor(MAX_JOURNAL_SEQUENCE, 1).unwrap();
    assert_eq!(
        near_limit.timestamp_ms(2).unwrap_err(),
        ClockError::Overflow
    );
    assert_eq!(
        VolatileClock::new()
            .set_anchor(MAX_JOURNAL_SEQUENCE + 1, 0)
            .unwrap_err(),
        ClockError::OutOfRange
    );
}
