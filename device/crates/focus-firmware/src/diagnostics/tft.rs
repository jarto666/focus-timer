//! Focused bring-up for the purchased 240x320 SPI ST7789V TFT module.

use embedded_hal::spi::MODE_3;
use esp_idf_svc::hal::{
    delay::FreeRtos,
    gpio::{AnyInputPin, Output, PinDriver},
    peripherals::Peripherals,
    spi::{Dma, SpiDeviceDriver, SpiDriver, SpiDriverConfig, config},
    units::MegaHertz,
};

const WIDTH: u16 = 240;
const HEIGHT: u16 = 320;
const FRAME_PAUSE_MS: u32 = 5;
const LOW_RES_SCALE: u16 = 4;
const LOW_RES_WIDTH: usize = WIDTH as usize / LOW_RES_SCALE as usize;
const LOW_RES_ROW_SAMPLES: usize = LOW_RES_WIDTH + 1;
const NOISE_SIZE: usize = 64;
const NOISE_MASK: usize = NOISE_SIZE - 1;
const ROWS_PER_TRANSFER: usize = 8;
const BYTES_PER_ROW: usize = WIDTH as usize * 2;

const BLACK: u16 = 0x0000;
const MUTED_WHITE: u16 = 0xdedf;
const TURQUOISE: u16 = 0x2efa;
const DIM_TURQUOISE: u16 = 0x1430;

const DIGIT_PATTERNS: [u8; 10] = [
    0b011_1111, 0b000_0110, 0b101_1011, 0b100_1111, 0b110_0110, 0b110_1101, 0b111_1101, 0b000_0111,
    0b111_1111, 0b110_1111,
];

const FOCUS_GLYPHS: [[u8; 5]; 5] = [
    [0b111_1111, 0b000_1001, 0b000_1001, 0b000_0001, 0b000_0001],
    [0b011_1110, 0b100_0001, 0b100_0001, 0b100_0001, 0b011_1110],
    [0b011_1110, 0b100_0001, 0b100_0001, 0b100_0001, 0b010_0010],
    [0b001_1111, 0b010_0000, 0b100_0000, 0b010_0000, 0b001_1111],
    [0b010_0110, 0b100_1001, 0b100_1001, 0b100_1001, 0b011_0010],
];

type Spi<'d> = SpiDeviceDriver<'d, SpiDriver<'d>>;

pub(super) fn run() -> ! {
    log::info!(
        "TFT diagnostic wiring: GND->GND VCC->3V3 SCL->GPIO6 SDA->GPIO7 RES->GPIO3 DC->GPIO4 CS->GPIO10 BLK->3V3"
    );

    let peripherals = Peripherals::take().expect("ESP32 peripherals must be available once");
    let mut reset =
        PinDriver::output(peripherals.pins.gpio3).expect("failed to configure TFT RES GPIO3");
    let mut dc =
        PinDriver::output(peripherals.pins.gpio4).expect("failed to configure TFT DC GPIO4");

    let spi_config = config::Config::new()
        .baudrate(MegaHertz(26).into())
        .data_mode(MODE_3)
        .write_only(true);
    let spi_driver_config =
        SpiDriverConfig::new().dma(Dma::Auto(BYTES_PER_ROW * ROWS_PER_TRANSFER));
    let mut spi = SpiDeviceDriver::new_single(
        peripherals.spi2,
        peripherals.pins.gpio6,
        peripherals.pins.gpio7,
        None::<AnyInputPin<'_>>,
        Some(peripherals.pins.gpio10),
        &spi_driver_config,
        &spi_config,
    )
    .expect("failed to initialize TFT SPI2 on GPIO6/GPIO7/GPIO10");

    initialize(&mut spi, &mut dc, &mut reset);
    log::info!("ST7789V initialized at 240x320 RGB565, SPI mode 3, 26 MHz");

    log::info!("Starting procedural mist UI; no framebuffer or video asset is allocated");
    let noise = build_noise_texture();
    let palette = build_mist_palette();
    let mut frame = 0_u32;
    loop {
        draw_mist_frame(&mut spi, &mut dc, frame, &noise, &palette);
        frame = frame.wrapping_add(1);
        if frame.is_multiple_of(60) {
            log::info!("TFT mist animation: {frame} frames rendered");
        }
        FreeRtos::delay_ms(FRAME_PAUSE_MS);
    }
}

fn initialize(
    spi: &mut Spi<'_>,
    dc: &mut PinDriver<'_, Output>,
    reset: &mut PinDriver<'_, Output>,
) {
    reset.set_high().expect("failed to release TFT reset");
    FreeRtos::delay_ms(10);
    reset.set_low().expect("failed to assert TFT reset");
    FreeRtos::delay_ms(20);
    reset.set_high().expect("failed to release TFT reset");
    FreeRtos::delay_ms(120);

    command(spi, dc, 0x01, &[]); // Software reset.
    FreeRtos::delay_ms(150);
    command(spi, dc, 0x11, &[]); // Sleep out.
    FreeRtos::delay_ms(120);
    command(spi, dc, 0x3a, &[0x55]); // 16-bit RGB565.
    command(spi, dc, 0x36, &[0x00]); // Portrait, RGB order.
    command(spi, dc, 0x21, &[]); // Display inversion on for common IPS panels.
    command(spi, dc, 0x13, &[]); // Normal display mode.
    FreeRtos::delay_ms(10);
    command(spi, dc, 0x29, &[]); // Display on.
    FreeRtos::delay_ms(120);
}

fn command(spi: &mut Spi<'_>, dc: &mut PinDriver<'_, Output>, value: u8, data: &[u8]) {
    dc.set_low().expect("failed to select TFT command mode");
    spi.write(&[value]).expect("failed to write TFT command");
    if !data.is_empty() {
        dc.set_high().expect("failed to select TFT data mode");
        spi.write(data).expect("failed to write TFT command data");
    }
}

fn set_window(
    spi: &mut Spi<'_>,
    dc: &mut PinDriver<'_, Output>,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) {
    assert!(width > 0 && height > 0, "TFT rectangle must be non-empty");
    let x_end = x + width - 1;
    let y_end = y + height - 1;
    assert!(
        x_end < WIDTH && y_end < HEIGHT,
        "TFT rectangle is out of bounds"
    );

    command(
        spi,
        dc,
        0x2a,
        &[(x >> 8) as u8, x as u8, (x_end >> 8) as u8, x_end as u8],
    );
    command(
        spi,
        dc,
        0x2b,
        &[(y >> 8) as u8, y as u8, (y_end >> 8) as u8, y_end as u8],
    );
    command(spi, dc, 0x2c, &[]);
}

fn draw_mist_frame(
    spi: &mut Spi<'_>,
    dc: &mut PinDriver<'_, Output>,
    frame: u32,
    noise: &[u8; NOISE_SIZE * NOISE_SIZE],
    palette: &[u16; 256],
) {
    set_window(spi, dc, 0, 0, WIDTH, HEIGHT);
    dc.set_high().expect("failed to select TFT pixel-data mode");

    let mut top_noise = [0_u8; LOW_RES_ROW_SAMPLES];
    let mut bottom_noise = [0_u8; LOW_RES_ROW_SAMPLES];
    let mut transfer = [0_u8; BYTES_PER_ROW * ROWS_PER_TRANSFER];

    for block_y in (0..HEIGHT).step_by(ROWS_PER_TRANSFER) {
        let rows = usize::from(HEIGHT - block_y).min(ROWS_PER_TRANSFER);
        for row in 0..rows {
            let y = block_y + row as u16;
            if y.is_multiple_of(LOW_RES_SCALE) {
                fill_noise_row(&mut top_noise, usize::from(y / LOW_RES_SCALE), frame, noise);
                fill_noise_row(
                    &mut bottom_noise,
                    usize::from(y / LOW_RES_SCALE) + 1,
                    frame,
                    noise,
                );
            }

            for x in 0..WIDTH {
                let low_x = usize::from(x / LOW_RES_SCALE);
                let fraction_x = (x % LOW_RES_SCALE) as u8;
                let fraction_y = (y % LOW_RES_SCALE) as u8;
                let top = blend_quarters(top_noise[low_x], top_noise[low_x + 1], fraction_x);
                let bottom =
                    blend_quarters(bottom_noise[low_x], bottom_noise[low_x + 1], fraction_x);
                let intensity = blend_quarters(top, bottom, fraction_y);
                let background = palette[usize::from(intensity)];
                let color = ui_color(x, y).unwrap_or(background);
                let [high, low] = color.to_be_bytes();
                let offset = row * BYTES_PER_ROW + usize::from(x) * 2;
                transfer[offset] = high;
                transfer[offset + 1] = low;
            }
        }
        spi.write(&transfer[..rows * BYTES_PER_ROW])
            .expect("failed to write TFT mist row block");
    }
}

fn fill_noise_row(
    row: &mut [u8; LOW_RES_ROW_SAMPLES],
    y: usize,
    frame: u32,
    noise: &[u8; NOISE_SIZE * NOISE_SIZE],
) {
    for (x, intensity) in row.iter_mut().enumerate() {
        *intensity = mist_intensity(x, y, frame, noise);
    }
}

#[inline]
fn blend_quarters(start: u8, end: u8, quarter: u8) -> u8 {
    let start_weight = u16::from(LOW_RES_SCALE as u8 - quarter);
    let end_weight = u16::from(quarter);
    ((u16::from(start) * start_weight + u16::from(end) * end_weight) / LOW_RES_SCALE) as u8
}

#[inline]
fn mist_intensity(x: usize, y: usize, frame: u32, noise: &[u8; NOISE_SIZE * NOISE_SIZE]) -> u8 {
    // Animate two differently-oriented samples from one seamless texture. The expensive
    // smoothing happens once during boot instead of three times for every pixel and frame.
    let drift = (frame / 2) as usize & NOISE_MASK;
    let slow = noise[((y + drift / 3) & NOISE_MASK) * NOISE_SIZE + ((x + drift) & NOISE_MASK)];
    let cross = noise[((x + drift / 4) & NOISE_MASK) * NOISE_SIZE
        + ((NOISE_MASK - (y & NOISE_MASK) + drift / 2) & NOISE_MASK)];
    let broad = noise[(((y / 2) + drift / 7) & NOISE_MASK) * NOISE_SIZE
        + (((x / 2) + NOISE_SIZE - (drift / 5 & NOISE_MASK)) & NOISE_MASK)];
    let mixed = (u16::from(slow) * 3 + u16::from(cross) * 2 + u16::from(broad) * 3) / 8;
    mixed as u8
}

fn build_noise_texture() -> [u8; NOISE_SIZE * NOISE_SIZE] {
    let mut noise = [0_u8; NOISE_SIZE * NOISE_SIZE];
    let mut scratch = [0_u8; NOISE_SIZE * NOISE_SIZE];

    for y in 0..NOISE_SIZE {
        for x in 0..NOISE_SIZE {
            noise[y * NOISE_SIZE + x] = hash(x as u32, y as u32, 0x73a5_1d09);
        }
    }

    // Repeated wrapped box blur creates a soft, tileable cloud field.
    for _ in 0..5 {
        for y in 0..NOISE_SIZE {
            for x in 0..NOISE_SIZE {
                let mut sum = 0_u16;
                for offset_y in [NOISE_MASK, 0, 1] {
                    for offset_x in [NOISE_MASK, 0, 1] {
                        let sample_y = (y + offset_y) & NOISE_MASK;
                        let sample_x = (x + offset_x) & NOISE_MASK;
                        sum += u16::from(noise[sample_y * NOISE_SIZE + sample_x]);
                    }
                }
                scratch[y * NOISE_SIZE + x] = (sum / 9) as u8;
            }
        }
        noise.copy_from_slice(&scratch);
    }

    noise
}

fn hash(x: u32, y: u32, seed: u32) -> u8 {
    let mut value = x
        .wrapping_mul(0x9e37_79b1)
        .wrapping_add(y.wrapping_mul(0x85eb_ca77))
        ^ seed;
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^= value >> 16;
    (value >> 24) as u8
}

fn build_mist_palette() -> [u16; 256] {
    let mut palette = [BLACK; 256];
    for (intensity, color) in palette.iter_mut().enumerate() {
        let mist = (intensity as u16).saturating_sub(72).saturating_mul(255) / 183;
        let red = 1 + mist * 3 / 255;
        let green = 4 + mist * 28 / 255;
        let blue = 9 + mist * 20 / 255;
        *color = rgb565(red as u8, green as u8, blue as u8);
    }
    palette
}

fn rgb565(red: u8, green: u8, blue: u8) -> u16 {
    (u16::from(red.min(31)) << 11) | (u16::from(green.min(63)) << 5) | u16::from(blue.min(31))
}

fn ui_color(x: u16, y: u16) -> Option<u16> {
    if (48..62).contains(&y) && glyph_text_pixel(x, y, 90, 48, 2) {
        return Some(TURQUOISE);
    }

    if (119..183).contains(&y) && timer_pixel(x, y) {
        return Some(MUTED_WHITE);
    }

    if (35..=204).contains(&x) && (232..=234).contains(&y) {
        return Some(if x <= 142 { TURQUOISE } else { DIM_TURQUOISE });
    }

    if (103..=136).contains(&x) && (256..=258).contains(&y) {
        return Some(TURQUOISE);
    }

    None
}

fn glyph_text_pixel(x: u16, y: u16, origin_x: u16, origin_y: u16, scale: u16) -> bool {
    if x < origin_x || y < origin_y {
        return false;
    }
    let local_x = x - origin_x;
    let local_y = y - origin_y;
    let character_width = 5 * scale;
    let stride = character_width + scale;
    let character = usize::from(local_x / stride);
    if character >= FOCUS_GLYPHS.len() || local_y >= 7 * scale {
        return false;
    }
    let column = usize::from((local_x % stride) / scale);
    if column >= 5 {
        return false;
    }
    let row = (local_y / scale) as u8;
    FOCUS_GLYPHS[character][column] & (1 << row) != 0
}

fn timer_pixel(x: u16, y: u16) -> bool {
    const DIGIT_Y: u16 = 119;
    const DIGIT_WIDTH: u16 = 32;
    const DIGIT_HEIGHT: u16 = 64;
    const THICKNESS: u16 = 5;

    let digit = match x {
        25..=56 => Some((2, 25)),
        67..=98 => Some((5, 67)),
        123..=154 => Some((0, 123)),
        165..=196 => Some((0, 165)),
        _ => None,
    };

    digit.is_some_and(|(digit, origin_x)| {
        seven_segment_pixel(
            x,
            y,
            origin_x,
            DIGIT_Y,
            DIGIT_WIDTH,
            DIGIT_HEIGHT,
            THICKNESS,
            digit,
        )
    }) || ((106..=112).contains(&x) && ((137..=143).contains(&y) || (159..=165).contains(&y)))
}

#[allow(clippy::too_many_arguments)]
fn seven_segment_pixel(
    x: u16,
    y: u16,
    origin_x: u16,
    origin_y: u16,
    width: u16,
    height: u16,
    thickness: u16,
    digit: usize,
) -> bool {
    if x < origin_x || y < origin_y || x >= origin_x + width || y >= origin_y + height {
        return false;
    }
    let x = x - origin_x;
    let y = y - origin_y;
    let half = height / 2;
    let pattern = DIGIT_PATTERNS[digit];

    let horizontal = x >= thickness && x < width - thickness;
    let vertical_left = x < thickness;
    let vertical_right = x >= width - thickness;

    (pattern & (1 << 0) != 0 && horizontal && y < thickness)
        || (pattern & (1 << 1) != 0 && vertical_right && y >= thickness && y < half)
        || (pattern & (1 << 2) != 0 && vertical_right && y >= half && y < height - thickness)
        || (pattern & (1 << 3) != 0 && horizontal && y >= height - thickness)
        || (pattern & (1 << 4) != 0 && vertical_left && y >= half && y < height - thickness)
        || (pattern & (1 << 5) != 0 && vertical_left && y >= thickness && y < half)
        || (pattern & (1 << 6) != 0
            && horizontal
            && y >= half - thickness / 2
            && y < half + thickness.div_ceil(2))
}
