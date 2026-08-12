//! Deliberately limited WS2812 bring-up mode for the purchased 24-pixel ring.
//!
//! This mode is selected explicitly with the `ring-diagnostic` Cargo feature.
//! It starts by sending an all-off frame and then blinks only pixel zero at
//! 32/255 blue so the single-pixel check remains visible without requesting a
//! meaningful load. `ring-full-diagnostic` holds all 24 pixels at clearly
//! visible white RGB(32,32,32), which stays inside the reviewed current cap.
//! With local decoupling installed, `ring-capped-diagnostic` cycles the actual
//! maximum-load Running, Paused, and Completed application frames.

#[cfg(all(feature = "ring-full-diagnostic", feature = "ring-capped-diagnostic"))]
compile_error!("select only one staged WS2812 diagnostic feature");

use core::time::Duration;
use std::thread;

use esp_idf_svc::hal::{
    peripherals::Peripherals,
    rmt::{
        PinState, Symbol, TxChannelDriver,
        config::{TransmitConfig, TxChannelConfig},
        encoder::CopyEncoder,
    },
    units::Hertz,
};
use esp_idf_svc::sys::EspError;
use focus_firmware::presentation::{Rgb, RingFrame};

const RMT_RESOLUTION: Hertz = Hertz(10_000_000);
const T0H: Duration = Duration::from_nanos(350);
const T0L: Duration = Duration::from_nanos(800);
const T1H: Duration = Duration::from_nanos(700);
const T1L: Duration = Duration::from_nanos(600);
const RESET: Duration = Duration::from_micros(300);
const PHASE_DURATION: Duration = Duration::from_secs(2);
#[cfg(all(
    not(feature = "ring-capped-diagnostic"),
    not(feature = "ring-full-diagnostic")
))]
const DIAGNOSTIC_BLUE: u8 = 32;
#[cfg(feature = "ring-full-diagnostic")]
const FULL_RING_WHITE: Rgb = Rgb {
    red: 32,
    green: 32,
    blue: 32,
};

pub fn run() -> ! {
    let peripherals = Peripherals::take().expect("ESP-IDF peripherals already taken");
    let mut channel = TxChannelDriver::new(
        peripherals.pins.gpio10,
        &TxChannelConfig {
            resolution: RMT_RESOLUTION,
            ..Default::default()
        },
    )
    .expect("failed to configure RMT output on GPIO10");

    let off = RingFrame::default();
    send_frame(&mut channel, &off).expect("failed to send startup-off WS2812 frame");
    log::info!("ring diagnostic ready on GPIO10: all 24 pixels commanded off");

    #[cfg(feature = "ring-full-diagnostic")]
    run_steady_full_ring(&mut channel);

    #[cfg(not(feature = "ring-full-diagnostic"))]
    thread::sleep(PHASE_DURATION);

    #[cfg(feature = "ring-capped-diagnostic")]
    run_capped_profile(&mut channel, &off);

    #[cfg(not(any(feature = "ring-capped-diagnostic", feature = "ring-full-diagnostic")))]
    run_single_pixel_stage(&mut channel, &off);
}

#[cfg(feature = "ring-full-diagnostic")]
fn run_steady_full_ring(channel: &mut TxChannelDriver<'_>) -> ! {
    let mut active = RingFrame::default();
    active.pixels.fill(FULL_RING_WHITE);

    loop {
        send_frame(channel, &active).expect("failed to send steady full-ring frame");
        log::info!("ring diagnostic: 24 pixels steady at RGB(32,32,32)");
        thread::sleep(PHASE_DURATION);
    }
}

#[cfg(not(any(feature = "ring-capped-diagnostic", feature = "ring-full-diagnostic")))]
fn run_single_pixel_stage(channel: &mut TxChannelDriver<'_>, off: &RingFrame) -> ! {
    let mut active = RingFrame::default();
    let dim_blue = Rgb {
        red: 0,
        green: 0,
        blue: DIAGNOSTIC_BLUE,
    };
    active.pixels[0] = dim_blue;

    loop {
        send_frame(channel, &active).expect("failed to send active WS2812 frame");
        log::info!("ring diagnostic: 1 pixel dim blue at {DIAGNOSTIC_BLUE}/255");
        thread::sleep(PHASE_DURATION);

        send_frame(channel, off).expect("failed to send all-off WS2812 frame");
        log::info!("ring diagnostic: all pixels off");
        thread::sleep(PHASE_DURATION);
    }
}

#[cfg(feature = "ring-capped-diagnostic")]
fn run_capped_profile(channel: &mut TxChannelDriver<'_>, off: &RingFrame) -> ! {
    let profiles = [
        ("Running", Rgb::RUNNING),
        ("Paused", Rgb::PAUSED),
        ("Completed", Rgb::COMPLETED),
    ];

    loop {
        for (name, color) in profiles {
            let mut frame = RingFrame::default();
            frame.pixels.fill(color);
            send_frame(channel, &frame).expect("failed to send capped WS2812 frame");
            log::info!(
                "ring capped diagnostic: {name}, 24 pixels at RGB({},{},{})",
                color.red,
                color.green,
                color.blue
            );
            thread::sleep(PHASE_DURATION);

            send_frame(channel, off).expect("failed to send all-off WS2812 frame");
            log::info!("ring capped diagnostic: all pixels off");
            thread::sleep(PHASE_DURATION);
        }
    }
}

fn send_frame(channel: &mut TxChannelDriver<'_>, frame: &RingFrame) -> Result<(), EspError> {
    let signal = encode_frame(frame)?;
    channel.send_and_wait(CopyEncoder::new()?, &signal, &TransmitConfig::default())
}

fn encode_frame(frame: &RingFrame) -> Result<Vec<Symbol>, EspError> {
    let zero = Symbol::new_with(RMT_RESOLUTION, PinState::High, T0H, PinState::Low, T0L)?;
    let one = Symbol::new_with(RMT_RESOLUTION, PinState::High, T1H, PinState::Low, T1L)?;
    let reset = Symbol::new_half_split(RMT_RESOLUTION, PinState::Low, PinState::Low, RESET)?;
    let mut signal = Vec::with_capacity(2 + frame.pixels.len() * 24);

    signal.push(reset);
    for pixel in frame.pixels {
        for byte in [pixel.green, pixel.red, pixel.blue] {
            for bit in (0..8).rev() {
                signal.push(if byte & (1 << bit) == 0 { zero } else { one });
            }
        }
    }
    signal.push(reset);

    Ok(signal)
}
