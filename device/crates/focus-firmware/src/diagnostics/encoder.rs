//! Polling EC11 bench diagnostic on the reviewed provisional GPIO map.

use std::time::Instant;

use esp_idf_svc::hal::{
    delay::FreeRtos,
    gpio::{PinDriver, Pull},
    peripherals::Peripherals,
};
use focus_core::InputEvent;
use focus_firmware::input::EncoderInput;

const POLL_INTERVAL_MS: u32 = 1;

#[derive(Default)]
struct EventCounts {
    left: u32,
    right: u32,
    press: u32,
    long_press: u32,
}

impl EventCounts {
    fn record(&mut self, event: InputEvent) {
        match event {
            InputEvent::RotateLeft => self.left = self.left.saturating_add(1),
            InputEvent::RotateRight => self.right = self.right.saturating_add(1),
            InputEvent::Press => self.press = self.press.saturating_add(1),
            InputEvent::LongPress => self.long_press = self.long_press.saturating_add(1),
            InputEvent::TimeAdvanced => return,
        }

        log::info!(
            "encoder event: {event:?}; totals left={}, right={}, press={}, long_press={}",
            self.left,
            self.right,
            self.press,
            self.long_press
        );
    }
}

pub(super) fn run() -> ! {
    let peripherals = Peripherals::take().expect("ESP-IDF peripherals already taken");
    let s1 = PinDriver::input(peripherals.pins.gpio0, Pull::Up)
        .expect("failed to configure EC11 S1 on GPIO0");
    let s2 = PinDriver::input(peripherals.pins.gpio4, Pull::Up)
        .expect("failed to configure EC11 S2 on GPIO4");
    let key = PinDriver::input(peripherals.pins.gpio5, Pull::Up)
        .expect("failed to configure EC11 KEY on GPIO5");

    let boot_s1 = s1.is_high();
    let boot_s2 = s2.is_high();
    let boot_key = key.is_high();
    // The purchased module's clockwise sequence is S1-leading. Feed S2 as the
    // decoder's A phase so the generic semantic convention remains
    // clockwise=RotateRight and counter-clockwise=RotateLeft.
    let mut input = EncoderInput::new(boot_s2, boot_s1, !boot_key);
    let mut counts = EventCounts::default();
    let started = Instant::now();

    log::info!(
        "encoder diagnostic ready with ESP pull-ups: module 5V label->3V3, S1->GPIO0, S2->GPIO4, KEY->GPIO5, GND->GND; boot levels S1={boot_s1}, S2={boot_s2}, KEY={boot_key}"
    );

    loop {
        let s1_high = s1.is_high();
        let s2_high = s2.is_high();
        let key_pressed = key.is_low();

        let now_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let events = input.sample(now_ms, s2_high, s1_high, key_pressed);
        if let Some(event) = events.rotation {
            counts.record(event);
        }
        if let Some(event) = events.button {
            counts.record(event);
        }
        FreeRtos::delay_ms(POLL_INTERVAL_MS);
    }
}
