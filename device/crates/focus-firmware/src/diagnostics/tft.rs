//! Focused bring-up for the purchased 240x320 SPI ST7789V TFT module.

use esp_idf_svc::hal::{delay::FreeRtos, peripherals::Peripherals};
use focus_core::{AppSnapshot, ViewState, default_catalog};

use focus_firmware::{
    presentation::{TftBackgroundMode, tft_view},
    tft_display::{FrameResult, TftDisplay},
};

pub(super) fn run() -> ! {
    log::info!(
        "TFT diagnostic wiring: GND->GND VCC->3V3 SCL->GPIO6 SDA->GPIO7 RES->GPIO3 DC->GPIO4 CS->GPIO10 BLK->3V3"
    );
    let peripherals = Peripherals::take().expect("ESP32 peripherals must be available once");
    let mut display = TftDisplay::new(
        peripherals.spi2,
        peripherals.pins.gpio6,
        peripherals.pins.gpio7,
        peripherals.pins.gpio10,
        peripherals.pins.gpio3,
        peripherals.pins.gpio4,
    )
    .expect("focused TFT diagnostic must initialize the verified harness");
    log::info!("ST7789V initialized at 240x320 RGB565, SPI mode 3, 26 MHz");

    let preset = default_catalog().preset(2);
    let view = tft_view(
        &AppSnapshot {
            state: ViewState::Running,
            remaining_ms: preset.duration_ms / 2,
            preset,
        },
        1,
    );
    let mut frame = 0_u32;
    loop {
        assert_eq!(
            display
                .render_frame(&view, TftBackgroundMode::Animated, || true)
                .expect("focused TFT diagnostic transfer must succeed"),
            FrameResult::Complete
        );
        frame = frame.wrapping_add(1);
        if frame.is_multiple_of(60) {
            log::info!("TFT shared-renderer diagnostic: {frame} frames rendered");
        }
        FreeRtos::delay_ms(5);
    }
}
