//! Repeating standalone active-buzzer test on GPIO1.

use std::time::Instant;

use esp_idf_svc::hal::{
    delay::FreeRtos,
    gpio::{Output, PinDriver},
    peripherals::Peripherals,
};
use focus_core::FeedbackPattern;
use focus_firmware::buzzer::BuzzerCadence;

const POLL_INTERVAL_MS: u32 = 5;
const FIRST_PATTERN_AT_MS: u64 = 2_000;
const BETWEEN_PATTERNS_MS: u64 = 2_000;

pub fn run() -> ! {
    let peripherals = Peripherals::take().expect("ESP-IDF peripherals already taken");
    let mut output =
        PinDriver::output(peripherals.pins.gpio1).expect("failed to configure buzzer GPIO1");
    output
        .set_low()
        .expect("failed to command buzzer off at startup");

    let started = Instant::now();
    let mut cadence = BuzzerCadence::new();
    let mut next_pattern = FeedbackPattern::Start;
    let mut next_trigger_ms = FIRST_PATTERN_AT_MS;

    log::info!(
        "buzzer diagnostic ready: GPIO1 -> 330 ohm -> active buzzer +; buzzer - -> GND; startup level=off"
    );

    loop {
        let now_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

        if !cadence.is_active() && now_ms >= next_trigger_ms {
            set_output(&mut output, cadence.trigger(now_ms, next_pattern));
            log::info!("buzzer diagnostic: {next_pattern:?} cadence started");
            next_pattern = match next_pattern {
                FeedbackPattern::Start => FeedbackPattern::Complete,
                FeedbackPattern::Complete => FeedbackPattern::Start,
            };
            next_trigger_ms = now_ms.saturating_add(BETWEEN_PATTERNS_MS);
        }

        if let Some(on) = cadence.update(now_ms) {
            set_output(&mut output, on);
            if !cadence.is_active() {
                log::info!("buzzer diagnostic: cadence complete, output off");
            }
        }

        FreeRtos::delay_ms(POLL_INTERVAL_MS);
    }
}

fn set_output(output: &mut PinDriver<'_, Output>, on: bool) {
    let result = if on {
        output.set_high()
    } else {
        output.set_low()
    };
    if let Err(error) = result {
        log::error!("buzzer GPIO update failed: {error:?}; diagnostic timing continues");
    }
}
