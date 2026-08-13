use focus_core::InputEvent;
use focus_firmware::input::{EncoderInput, LONG_PRESS_MS, QuadratureDecoder, SwitchClassifier};

fn collect_rotation(decoder: &mut QuadratureDecoder, samples: &[(bool, bool)]) -> Vec<InputEvent> {
    samples
        .iter()
        .filter_map(|&(a, b)| decoder.sample(a, b))
        .collect()
}

#[test]
fn full_detents_emit_exactly_one_event_in_each_direction() {
    let mut right = QuadratureDecoder::new(true, true);
    assert_eq!(
        collect_rotation(
            &mut right,
            &[(true, false), (false, false), (false, true), (true, true)]
        ),
        [InputEvent::RotateRight]
    );

    let mut left = QuadratureDecoder::new(true, true);
    assert_eq!(
        collect_rotation(
            &mut left,
            &[(false, true), (false, false), (true, false), (true, true)]
        ),
        [InputEvent::RotateLeft]
    );
}

#[test]
fn quadrature_contact_bounce_is_coalesced() {
    let mut decoder = QuadratureDecoder::new(true, true);
    let samples = [
        (true, false),
        (true, true),
        (true, false),
        (false, false),
        (true, false),
        (false, false),
        (false, true),
        (false, false),
        (false, true),
        (true, true),
        (true, true),
    ];
    assert_eq!(
        collect_rotation(&mut decoder, &samples),
        [InputEvent::RotateRight]
    );
}

#[test]
fn ambiguous_two_bit_jump_does_not_create_a_partial_detent() {
    let mut decoder = QuadratureDecoder::new(true, true);
    let ambiguous_then_half = [(false, false), (false, true), (true, true)];
    assert!(collect_rotation(&mut decoder, &ambiguous_then_half).is_empty());

    let valid = [(true, false), (false, false), (false, true), (true, true)];
    assert_eq!(
        collect_rotation(&mut decoder, &valid),
        [InputEvent::RotateRight]
    );
}

#[test]
fn short_press_is_emitted_once_after_debounced_release() {
    let mut button = SwitchClassifier::new(false);
    assert_eq!(button.sample(10, true), None);
    assert_eq!(button.sample(34, true), None);
    assert_eq!(button.sample(35, true), None);
    assert_eq!(button.sample(100, false), None);
    assert_eq!(button.sample(124, false), None);
    assert_eq!(button.sample(125, false), Some(InputEvent::Press));
    assert_eq!(button.sample(200, false), None);
}

#[test]
fn switch_bounce_produces_one_short_press() {
    let mut button = SwitchClassifier::new(false);
    let samples = [
        (10, true),
        (15, false),
        (20, true),
        (44, true),
        (45, true),
        (100, false),
        (105, true),
        (110, false),
        (134, false),
        (135, false),
    ];
    let events: Vec<_> = samples
        .into_iter()
        .filter_map(|(now, pressed)| button.sample(now, pressed))
        .collect();
    assert_eq!(events, [InputEvent::Press]);
}

#[test]
fn held_switch_emits_only_long_press() {
    let mut button = SwitchClassifier::new(false);
    assert_eq!(button.sample(10, true), None);
    assert_eq!(button.sample(35, true), None);
    assert_eq!(button.sample(10 + LONG_PRESS_MS - 1, true), None);
    assert_eq!(
        button.sample(10 + LONG_PRESS_MS, true),
        Some(InputEvent::LongPress)
    );
    assert_eq!(button.sample(900, true), None);
    assert_eq!(button.sample(901, false), None);
    assert_eq!(button.sample(926, false), None);
}

#[test]
fn release_sample_can_finish_a_long_press_without_duplicate_short_press() {
    let mut button = SwitchClassifier::new(false);
    button.sample(10, true);
    button.sample(35, true);
    button.sample(10 + LONG_PRESS_MS, false);
    assert_eq!(
        button.sample(10 + LONG_PRESS_MS + 25, false),
        Some(InputEvent::LongPress)
    );
    assert_eq!(button.sample(900, false), None);
}

#[test]
fn button_held_during_boot_is_ignored_until_rearmed() {
    let mut button = SwitchClassifier::new(true);
    assert_eq!(button.sample(LONG_PRESS_MS * 2, true), None);
    assert_eq!(button.sample(LONG_PRESS_MS * 2 + 1, false), None);
    assert_eq!(button.sample(LONG_PRESS_MS * 2 + 26, false), None);

    assert_eq!(button.sample(2_000, true), None);
    assert_eq!(button.sample(2_025, true), None);
    assert_eq!(button.sample(2_100, false), None);
    assert_eq!(button.sample(2_125, false), Some(InputEvent::Press));
}

#[test]
fn combined_adapter_can_report_rotation_and_button_without_state_mutation() {
    let mut input = EncoderInput::new(true, true, false);
    input.sample(0, true, false, false);
    input.sample(1, false, false, false);
    input.sample(2, false, true, false);
    let events = input.sample(3, true, true, true);

    assert_eq!(events.rotation, Some(InputEvent::RotateRight));
    assert_eq!(events.button, None);
}
