//! Platform-independent, bounded RGB565 composition for the 240x320 TFT.

use core::convert::Infallible;

use embedded_graphics::{
    Pixel,
    geometry::{OriginDimensions, Point, Size},
    mono_font::{
        MonoTextStyle,
        ascii::{FONT_6X10, FONT_10X20},
    },
    pixelcolor::{IntoStorage, Rgb565},
    prelude::{DrawTarget, Drawable, Primitive},
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};

use crate::presentation::{TFT_PROGRESS_MAX, TftBackgroundMode, TftScene, TftView};

pub const TFT_WIDTH: u16 = 240;
pub const TFT_HEIGHT: u16 = 320;
pub const TFT_ROWS_PER_TRANSFER: usize = 8;
pub const TFT_BYTES_PER_ROW: usize = TFT_WIDTH as usize * 2;
pub const TFT_TRANSFER_BYTES: usize = TFT_BYTES_PER_ROW * TFT_ROWS_PER_TRANSFER;

const LOW_RES_SCALE: u16 = 4;
const LOW_RES_WIDTH: usize = TFT_WIDTH as usize / LOW_RES_SCALE as usize;
const LOW_RES_ROW_SAMPLES: usize = LOW_RES_WIDTH + 1;
const NOISE_SIZE: usize = 64;
const NOISE_MASK: usize = NOISE_SIZE - 1;

const BLACK: u16 = 0x0000;
const MUTED_WHITE: Rgb565 = Rgb565::new(27, 55, 27);
const TURQUOISE: Rgb565 = Rgb565::new(5, 58, 26);
const DIM_TURQUOISE: Rgb565 = Rgb565::new(2, 24, 18);
const NAVY_PANEL: Rgb565 = Rgb565::new(1, 10, 12);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TftRenderError {
    InvalidBlock,
    BufferTooSmall,
}

/// Owns only disposable visual state and fixed-size rendering resources.
pub struct TftRenderer {
    noise: [u8; NOISE_SIZE * NOISE_SIZE],
    palette: [u16; 256],
    phase: u32,
}

impl Default for TftRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TftRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            noise: build_noise_texture(),
            palette: build_mist_palette(),
            phase: 0,
        }
    }

    #[must_use]
    pub const fn phase(&self) -> u32 {
        self.phase
    }

    pub fn advance(&mut self, background: TftBackgroundMode) {
        if background == TftBackgroundMode::Animated {
            self.phase = self.phase.wrapping_add(1);
        }
    }

    /// Composes one bounded row block and returns its byte count.
    ///
    /// # Errors
    ///
    /// Returns an error for an out-of-range/empty block or an undersized output buffer.
    pub fn render_block(
        &self,
        view: &TftView,
        background: TftBackgroundMode,
        block_y: u16,
        rows: usize,
        output: &mut [u8],
    ) -> Result<usize, TftRenderError> {
        if rows == 0
            || rows > TFT_ROWS_PER_TRANSFER
            || block_y >= TFT_HEIGHT
            || usize::from(TFT_HEIGHT - block_y) < rows
        {
            return Err(TftRenderError::InvalidBlock);
        }
        let byte_count = rows * TFT_BYTES_PER_ROW;
        if output.len() < byte_count {
            return Err(TftRenderError::BufferTooSmall);
        }

        let phase = if background == TftBackgroundMode::Animated {
            self.phase
        } else {
            0
        };
        let mut top_noise = [0_u8; LOW_RES_ROW_SAMPLES];
        let mut bottom_noise = [0_u8; LOW_RES_ROW_SAMPLES];

        for row in 0..rows {
            let y = block_y + u16::try_from(row).unwrap_or(0);
            fill_noise_row(
                &mut top_noise,
                usize::from(y / LOW_RES_SCALE),
                phase,
                &self.noise,
            );
            fill_noise_row(
                &mut bottom_noise,
                usize::from(y / LOW_RES_SCALE) + 1,
                phase,
                &self.noise,
            );
            for x in 0..TFT_WIDTH {
                let low_x = usize::from(x / LOW_RES_SCALE);
                let fraction_x = (x % LOW_RES_SCALE) as u8;
                let fraction_y = (y % LOW_RES_SCALE) as u8;
                let top = blend_quarters(top_noise[low_x], top_noise[low_x + 1], fraction_x);
                let bottom =
                    blend_quarters(bottom_noise[low_x], bottom_noise[low_x + 1], fraction_x);
                let intensity = blend_quarters(top, bottom, fraction_y);
                put_pixel(output, row, x, self.palette[usize::from(intensity)]);
            }
        }

        compose_foreground(view, BlockTarget::new(output, block_y, rows));
        Ok(byte_count)
    }

    #[must_use]
    pub fn wrapped_sample(&self, x: usize, y: usize, phase: u32) -> u8 {
        mist_intensity(x, y, phase, &self.noise)
    }

    #[must_use]
    pub const fn palette_color(&self, index: u8) -> u16 {
        self.palette[index as usize]
    }
}

fn compose_foreground(view: &TftView, mut target: BlockTarget<'_>) {
    let small = MonoTextStyle::new(&FONT_6X10, MUTED_WHITE);
    let accent = MonoTextStyle::new(&FONT_6X10, TURQUOISE);
    let heading = MonoTextStyle::new(&FONT_10X20, TURQUOISE);
    let large = MonoTextStyle::new(&FONT_10X20, MUTED_WHITE);

    if target.intersects(24, 46) {
        let _ = Text::with_alignment(
            view.state_label,
            Point::new(120, 45),
            heading,
            Alignment::Center,
        )
        .draw(&mut target);
    }
    if target.intersects(70, 84) {
        let _ = Text::with_alignment(
            view.preset_name.as_str(),
            Point::new(120, 82),
            small,
            Alignment::Center,
        )
        .draw(&mut target);
    }
    if target.intersects(136, 160) {
        let _ = Text::with_alignment(
            view.time.as_str(),
            Point::new(120, 158),
            large,
            Alignment::Center,
        )
        .draw(&mut target);
    }

    if target.intersects(205, 216) {
        let outline = Rectangle::new(Point::new(20, 205), Size::new(200, 10));
        let _ = outline
            .into_styled(PrimitiveStyle::with_stroke(DIM_TURQUOISE, 1))
            .draw(&mut target);
        let fill_width =
            u32::from(view.progress.min(TFT_PROGRESS_MAX)) * 198 / u32::from(TFT_PROGRESS_MAX);
        if fill_width > 0 {
            let _ = Rectangle::new(Point::new(21, 206), Size::new(fill_width, 8))
                .into_styled(PrimitiveStyle::with_fill(TURQUOISE))
                .draw(&mut target);
        }
    }

    if view.scene == TftScene::CatalogConfirmation && target.intersects(228, 269) {
        let _ = Rectangle::new(Point::new(20, 228), Size::new(200, 40))
            .into_styled(PrimitiveStyle::with_fill(NAVY_PANEL))
            .draw(&mut target);
    }
    if target.intersects(240, 254) {
        let _ = Text::with_alignment(view.hint, Point::new(120, 252), accent, Alignment::Center)
            .draw(&mut target);
    }
}

struct BlockTarget<'a> {
    output: &'a mut [u8],
    block_y: u16,
    rows: usize,
}

impl<'a> BlockTarget<'a> {
    fn new(output: &'a mut [u8], block_y: u16, rows: usize) -> Self {
        Self {
            output,
            block_y,
            rows,
        }
    }

    fn intersects(&self, start_y: u16, end_y: u16) -> bool {
        let block_end = self.block_y + u16::try_from(self.rows).unwrap_or(0);
        self.block_y < end_y && block_end > start_y
    }
}

impl OriginDimensions for BlockTarget<'_> {
    fn size(&self) -> Size {
        Size::new(u32::from(TFT_WIDTH), u32::from(TFT_HEIGHT))
    }
}

impl DrawTarget for BlockTarget<'_> {
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let block_end = self.block_y + u16::try_from(self.rows).unwrap_or(0);
        for Pixel(point, color) in pixels {
            let Ok(x) = u16::try_from(point.x) else {
                continue;
            };
            let Ok(y) = u16::try_from(point.y) else {
                continue;
            };
            if x < TFT_WIDTH && y >= self.block_y && y < block_end {
                put_pixel(
                    self.output,
                    usize::from(y - self.block_y),
                    x,
                    color.into_storage(),
                );
            }
        }
        Ok(())
    }
}

fn put_pixel(output: &mut [u8], row: usize, x: u16, color: u16) {
    let offset = row * TFT_BYTES_PER_ROW + usize::from(x) * 2;
    let [high, low] = color.to_be_bytes();
    output[offset] = high;
    output[offset + 1] = low;
}

fn fill_noise_row(
    row: &mut [u8; LOW_RES_ROW_SAMPLES],
    y: usize,
    phase: u32,
    noise: &[u8; NOISE_SIZE * NOISE_SIZE],
) {
    for (x, intensity) in row.iter_mut().enumerate() {
        *intensity = mist_intensity(x, y, phase, noise);
    }
}

#[inline]
fn blend_quarters(start: u8, end: u8, quarter: u8) -> u8 {
    let scale = u8::try_from(LOW_RES_SCALE).expect("low-resolution scale fits in u8");
    let start_weight = u16::from(scale - quarter);
    let end_weight = u16::from(quarter);
    u8::try_from((u16::from(start) * start_weight + u16::from(end) * end_weight) / LOW_RES_SCALE)
        .expect("weighted u8 average fits in u8")
}

#[inline]
fn mist_intensity(x: usize, y: usize, phase: u32, noise: &[u8; NOISE_SIZE * NOISE_SIZE]) -> u8 {
    let drift = (phase / 2) as usize & NOISE_MASK;
    let slow = noise[((y + drift / 3) & NOISE_MASK) * NOISE_SIZE + ((x + drift) & NOISE_MASK)];
    let cross = noise[((x + drift / 4) & NOISE_MASK) * NOISE_SIZE
        + ((NOISE_MASK - (y & NOISE_MASK) + drift / 2) & NOISE_MASK)];
    let broad = noise[(((y / 2) + drift / 7) & NOISE_MASK) * NOISE_SIZE
        + (((x / 2) + NOISE_SIZE - ((drift / 5) & NOISE_MASK)) & NOISE_MASK)];
    u8::try_from((u16::from(slow) * 3 + u16::from(cross) * 2 + u16::from(broad) * 3) / 8)
        .expect("weighted u8 average fits in u8")
}

fn build_noise_texture() -> [u8; NOISE_SIZE * NOISE_SIZE] {
    let mut noise = [0_u8; NOISE_SIZE * NOISE_SIZE];
    let mut scratch = [0_u8; NOISE_SIZE * NOISE_SIZE];
    for y in 0..NOISE_SIZE {
        for x in 0..NOISE_SIZE {
            noise[y * NOISE_SIZE + x] = hash(
                u32::try_from(x).expect("noise x coordinate fits in u32"),
                u32::try_from(y).expect("noise y coordinate fits in u32"),
                0x73a5_1d09,
            );
        }
    }
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
                scratch[y * NOISE_SIZE + x] =
                    u8::try_from(sum / 9).expect("averaged noise value fits in u8");
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
    value.to_be_bytes()[0]
}

fn build_mist_palette() -> [u16; 256] {
    let mut palette = [BLACK; 256];
    for (intensity, color) in palette.iter_mut().enumerate() {
        let mist = u16::try_from(intensity)
            .expect("palette index fits in u16")
            .saturating_sub(72)
            .saturating_mul(255)
            / 183;
        let red = 1 + mist * 3 / 255;
        let green = 4 + mist * 28 / 255;
        let blue = 9 + mist * 20 / 255;
        *color = rgb565(
            u8::try_from(red).expect("red channel fits in u8"),
            u8::try_from(green).expect("green channel fits in u8"),
            u8::try_from(blue).expect("blue channel fits in u8"),
        );
    }
    palette
}

fn rgb565(red: u8, green: u8, blue: u8) -> u16 {
    u16::from(red.min(31)) << 11 | u16::from(green.min(63)) << 5 | u16::from(blue.min(31))
}
