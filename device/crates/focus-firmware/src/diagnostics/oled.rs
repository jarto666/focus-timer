//! Focused bring-up for the purchased 128x64 I2C SSD1306 module.

use embedded_graphics::{
    Drawable,
    mono_font::{
        MonoTextStyle,
        ascii::{FONT_6X10, FONT_10X20},
    },
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Point},
    text::{Baseline, Text},
};
use esp_idf_svc::hal::{
    delay::{BLOCK, FreeRtos},
    i2c::{I2cConfig, I2cDriver},
    peripherals::Peripherals,
    units::KiloHertz,
};
use focus_core::{AppSnapshot, ViewState, default_catalog};
use ssd1306::{
    I2CDisplayInterface, Ssd1306,
    mode::DisplayConfig,
    prelude::{DisplayRotation, DisplaySize128x64},
};

use focus_firmware::presentation::{OLED_LAYOUT, OledView, oled_view};

const CANDIDATE_ADDRESSES: [u8; 2] = [0x3c, 0x3d];
const FRAME_DURATION_MS: u32 = 3_000;

pub(super) fn run() -> ! {
    log::info!("OLED diagnostic: GND->GND, VDD->3V3, SCK/SCL->GPIO6, SDA->GPIO7; 100 kHz I2C");

    let peripherals = Peripherals::take().expect("ESP32 peripherals must be available once");
    // The first physical run with both controller pull-ups disabled timed out
    // at 0x3C and 0x3D. Enable the ESP32's 3.3 V-domain pull-ups for the second
    // stage so the module can be distinguished from an absent/weak pull-up
    // network without adding external components yet.
    let config = I2cConfig::new()
        .baudrate(KiloHertz(100).into())
        .sda_enable_pullup(true)
        .scl_enable_pullup(true);
    let mut i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio7,
        peripherals.pins.gpio6,
        &config,
    )
    .expect("GPIO6/GPIO7 I2C initialization must succeed");

    let address = detect_address(&mut i2c);
    log::info!("OLED ACK at I2C address 0x{address:02X}");

    let interface = I2CDisplayInterface::new_custom_address(i2c, address);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().expect("SSD1306 initialization must succeed");

    let preset = default_catalog().preset(2);
    let duration_ms = preset.duration_ms;
    let snapshots = [
        AppSnapshot {
            state: ViewState::Idle,
            preset: preset.clone(),
            remaining_ms: duration_ms,
        },
        AppSnapshot {
            state: ViewState::Running,
            preset: preset.clone(),
            remaining_ms: 24 * 60_000 + 37_000,
        },
        AppSnapshot {
            state: ViewState::Paused,
            preset: preset.clone(),
            remaining_ms: 24 * 60_000 + 37_000,
        },
        AppSnapshot {
            state: ViewState::Completed,
            preset,
            remaining_ms: 0,
        },
    ];

    log::info!(
        "OLED initialized: 128x64, rotation=0, controller 3V3 pull-ups enabled; cycling READY/FOCUS/PAUSED/COMPLETE every 3 seconds"
    );
    loop {
        for snapshot in &snapshots {
            let view = oled_view(snapshot.clone());
            display.clear_buffer();
            draw_view(&mut display, &view).expect("drawing into the OLED buffer must succeed");
            display.flush().expect("OLED frame transfer must succeed");
            log::info!(
                "OLED frame: address=0x{address:02X} state={} preset={} time={}",
                view.state_label,
                view.preset_name,
                view.time.as_str()
            );
            FreeRtos::delay_ms(FRAME_DURATION_MS);
        }
    }
}

fn detect_address(i2c: &mut I2cDriver<'_>) -> u8 {
    let mut detected = None;
    for address in CANDIDATE_ADDRESSES {
        // 0x00 selects the command stream; 0xAE is the harmless display-off
        // command. An ACK confirms both power and the I2C address.
        match i2c.write(address, &[0x00, 0xae], BLOCK) {
            Ok(()) => {
                assert!(
                    detected.is_none(),
                    "more than one SSD1306 candidate address acknowledged"
                );
                detected = Some(address);
            }
            Err(error) => log::info!("no OLED ACK at 0x{address:02X}: {error}"),
        }
    }

    detected.expect("OLED did not ACK at 0x3C or 0x3D; check 3V3, GND, SCK/SCL, and SDA")
}

fn draw_view<D>(target: &mut D, view: &OledView) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let small = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let large = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);

    Text::with_baseline(
        view.state_label,
        Point::new(0, i32::from(OLED_LAYOUT.state_y)),
        small,
        Baseline::Top,
    )
    .draw(target)?;
    Text::with_baseline(
        view.preset_name.as_str(),
        Point::new(0, i32::from(OLED_LAYOUT.preset_y)),
        small,
        Baseline::Top,
    )
    .draw(target)?;
    Text::with_baseline(
        view.time.as_str(),
        Point::new(0, i32::from(OLED_LAYOUT.time_y)),
        large,
        Baseline::Top,
    )
    .draw(target)?;
    Text::with_baseline(
        view.hint,
        Point::new(0, i32::from(OLED_LAYOUT.hint_y)),
        small,
        Baseline::Top,
    )
    .draw(target)?;

    Ok(())
}
