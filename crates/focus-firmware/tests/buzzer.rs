use focus_core::FeedbackPattern;
use focus_firmware::buzzer::BuzzerCadence;

#[test]
fn start_is_one_short_pulse() {
    let mut cadence = BuzzerCadence::new();

    assert!(cadence.trigger(1_000, FeedbackPattern::Start));
    assert!(cadence.is_active());
    assert_eq!(cadence.update(1_079), None);
    assert_eq!(cadence.update(1_080), Some(false));
    assert!(!cadence.is_active());
}

#[test]
fn completion_is_three_distinct_pulses() {
    let mut cadence = BuzzerCadence::new();

    assert!(cadence.trigger(0, FeedbackPattern::Complete));
    assert_eq!(cadence.update(120), Some(false));
    assert_eq!(cadence.update(220), Some(true));
    assert_eq!(cadence.update(340), Some(false));
    assert_eq!(cadence.update(440), Some(true));
    assert_eq!(cadence.update(719), None);
    assert_eq!(cadence.update(720), Some(false));
    assert!(!cadence.is_active());
}

#[test]
fn a_new_pattern_replaces_the_old_one_without_blocking() {
    let mut cadence = BuzzerCadence::new();

    let _ = cadence.trigger(0, FeedbackPattern::Complete);
    assert!(cadence.trigger(50, FeedbackPattern::Start));
    assert_eq!(cadence.update(129), None);
    assert_eq!(cadence.update(130), Some(false));
    assert!(!cadence.is_active());
}
