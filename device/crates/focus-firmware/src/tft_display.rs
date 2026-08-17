//! ESP-IDF transport for the shared ST7789 renderer.

extern crate std;

use std::{
    thread,
    time::{Duration, Instant},
};

use embedded_hal::spi::MODE_3;
use esp_idf_svc::hal::{
    delay::FreeRtos,
    gpio::{AnyInputPin, Output, OutputPin, PinDriver},
    spi::{Dma, SpiAnyPins, SpiDeviceDriver, SpiDriver, SpiDriverConfig, config},
    task::{self, thread::ThreadSpawnConfiguration},
    units::MegaHertz,
};

use crate::{
    display_worker::{DisplayHealth, LatestViewSlot},
    presentation::{TftBackgroundMode, TftRenderSignals, TftView, tft_render_policy},
    tft_render::{
        TFT_BYTES_PER_ROW, TFT_HEIGHT, TFT_ROWS_PER_TRANSFER, TFT_TRANSFER_BYTES, TFT_WIDTH,
        TftRenderError, TftRenderer,
    },
};

const WORKER_POLL_INTERVAL: Duration = Duration::from_millis(10);
const WORKER_STACK_SIZE: usize = 32 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TftWorkerStartError {
    ThreadConfiguration,
    ThreadSpawn,
}

/// Starts the best-effort TFT owner and returns its non-blocking publication slot.
///
/// # Errors
///
/// Returns when the low-priority thread cannot be configured or spawned. The
/// caller can continue as a fully functional headless timer.
pub fn spawn_tft_worker<SPI, SCLK, MOSI, CS, RESET, DC>(
    spi: SPI,
    sclk: SCLK,
    mosi: MOSI,
    cs: CS,
    reset: RESET,
    dc: DC,
) -> Result<LatestViewSlot, TftWorkerStartError>
where
    SPI: SpiAnyPins + Send + 'static,
    SCLK: OutputPin + Send + 'static,
    MOSI: OutputPin + Send + 'static,
    CS: OutputPin + Send + 'static,
    RESET: OutputPin + Send + 'static,
    DC: OutputPin + Send + 'static,
{
    let slot = LatestViewSlot::new();
    let worker_slot = slot.clone();
    let original_config = ThreadSpawnConfiguration::get().unwrap_or_default();
    let worker_config = ThreadSpawnConfiguration {
        name: Some(c"focus-tft"),
        stack_size: WORKER_STACK_SIZE,
        priority: 1,
        inherit: false,
        ..ThreadSpawnConfiguration::default()
    };
    worker_config
        .set()
        .map_err(|_| TftWorkerStartError::ThreadConfiguration)?;
    let spawned = thread::Builder::new()
        .name("focus-tft".into())
        .stack_size(WORKER_STACK_SIZE)
        .spawn(move || display_worker_loop(worker_slot, spi, sclk, mosi, cs, reset, dc));
    let restore_result = original_config.set();
    if restore_result.is_err() {
        log::warn!("could not restore parent pthread configuration after TFT worker spawn");
    }
    spawned.map_err(|_| TftWorkerStartError::ThreadSpawn)?;
    Ok(slot)
}

fn display_worker_loop<SPI, SCLK, MOSI, CS, RESET, DC>(
    slot: LatestViewSlot,
    spi: SPI,
    sclk: SCLK,
    mosi: MOSI,
    cs: CS,
    reset: RESET,
    dc: DC,
) where
    SPI: SpiAnyPins + 'static,
    SCLK: OutputPin + 'static,
    MOSI: OutputPin + 'static,
    CS: OutputPin + 'static,
    RESET: OutputPin + 'static,
    DC: OutputPin + 'static,
{
    let mut display = match TftDisplay::new(spi, sclk, mosi, cs, reset, dc) {
        Ok(display) => {
            log::info!(
                "TFT worker initialized: 240x320 mode-3 26MHz priority=1 stack={WORKER_STACK_SIZE}"
            );
            display
        }
        Err(error) => {
            log::error!("TFT worker initialization failed: {error:?}; continuing headless");
            loop {
                let _ = slot.try_take();
                thread::sleep(Duration::from_secs(1));
            }
        }
    };
    let mut health = DisplayHealth::default();
    let mut current: Option<TftView> = None;
    let mut recent_interaction = false;
    let mut last_frame_started: Option<Instant> = None;
    let mut previous_frame_delayed = false;
    let mut fault_injected = false;
    let started = Instant::now();
    #[cfg(feature = "acceptance-diagnostic")]
    let mut resource_snapshot_logged = false;

    loop {
        if let Some(newest) = slot.try_take() {
            current = Some(newest);
            recent_interaction = true;
        }
        let Some(view) = current.as_ref() else {
            thread::sleep(WORKER_POLL_INTERVAL);
            continue;
        };
        let now_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        if !health.retry_due(now_ms) {
            thread::sleep(WORKER_POLL_INTERVAL);
            continue;
        }
        let policy = tft_render_policy(
            view.scene,
            TftRenderSignals {
                recent_interaction,
                frame_delayed: previous_frame_delayed,
                consecutive_failures: health.consecutive_failures(),
            },
        );
        if !policy.render_immediately
            && last_frame_started.is_some_and(|last| {
                last.elapsed() < Duration::from_millis(u64::from(policy.next_frame_after_ms))
            })
        {
            thread::sleep(WORKER_POLL_INTERVAL);
            continue;
        }

        let generation = view.generation;
        let frame_started = Instant::now();
        last_frame_started = Some(frame_started);
        let render_result = if cfg!(feature = "tft-failure-diagnostic") && !fault_injected {
            fault_injected = true;
            Err(TftDisplayError::InjectedTransfer)
        } else {
            display.render_frame(view, policy.background, || {
                !slot.try_has_newer_than(generation).unwrap_or(false)
            })
        };
        match render_result {
            Ok(FrameResult::Complete) => {
                health.record_success();
                previous_frame_delayed = frame_started.elapsed()
                    > Duration::from_millis(u64::from(policy.next_frame_after_ms));
                recent_interaction = false;
                #[cfg(feature = "acceptance-diagnostic")]
                if !resource_snapshot_logged {
                    let snapshot = focus_esp_resources::snapshot();
                    log::info!(
                        "TFT worker resource snapshot: heap_free_8bit={} heap_minimum_8bit={} worker_stack_minimum_free={} frame_ms={}",
                        snapshot.heap_free_8bit,
                        snapshot.heap_minimum_8bit,
                        snapshot.current_stack_minimum_free,
                        frame_started.elapsed().as_millis()
                    );
                    resource_snapshot_logged = true;
                }
            }
            Ok(FrameResult::Superseded) => {
                if let Some(newest) = slot.try_take() {
                    current = Some(newest);
                }
                recent_interaction = true;
            }
            Err(error) => {
                let now_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                if health.record_failure(now_ms) {
                    log::error!(
                        "TFT transfer failed: {error:?}; retaining newest view and retrying with bounded backoff"
                    );
                }
                previous_frame_delayed = true;
                recent_interaction = false;
            }
        }
    }
}

type Spi<'d> = SpiDeviceDriver<'d, SpiDriver<'d>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TftDisplayError {
    ResetPin,
    DataCommandPin,
    SpiInitialization,
    SpiTransfer,
    InvalidWindow,
    Rendering(TftRenderError),
    InjectedTransfer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameResult {
    Complete,
    Superseded,
}

pub struct TftDisplay<'d> {
    spi: Spi<'d>,
    dc: PinDriver<'d, Output>,
    reset: PinDriver<'d, Output>,
    renderer: TftRenderer,
}

impl<'d> TftDisplay<'d> {
    /// Configures and initializes a portrait 240x320 ST7789V display.
    ///
    /// # Errors
    ///
    /// Returns a typed stage error; production callers may continue headless.
    pub fn new<SPI, SCLK, MOSI, CS, RESET, DC>(
        spi: SPI,
        sclk: SCLK,
        mosi: MOSI,
        cs: CS,
        reset: RESET,
        dc: DC,
    ) -> Result<Self, TftDisplayError>
    where
        SPI: SpiAnyPins + 'd,
        SCLK: OutputPin + 'd,
        MOSI: OutputPin + 'd,
        CS: OutputPin + 'd,
        RESET: OutputPin + 'd,
        DC: OutputPin + 'd,
    {
        let reset = PinDriver::output(reset).map_err(|_| TftDisplayError::ResetPin)?;
        let dc = PinDriver::output(dc).map_err(|_| TftDisplayError::DataCommandPin)?;
        let spi_config = config::Config::new()
            .baudrate(MegaHertz(26).into())
            .data_mode(MODE_3)
            .write_only(true);
        let spi_driver_config =
            SpiDriverConfig::new().dma(Dma::Auto(TFT_BYTES_PER_ROW * TFT_ROWS_PER_TRANSFER));
        let spi = SpiDeviceDriver::new_single(
            spi,
            sclk,
            mosi,
            None::<AnyInputPin<'d>>,
            Some(cs),
            &spi_driver_config,
            &spi_config,
        )
        .map_err(|_| TftDisplayError::SpiInitialization)?;
        let mut display = Self {
            spi,
            dc,
            reset,
            renderer: TftRenderer::new(),
        };
        display.initialize()?;
        Ok(display)
    }

    /// Renders row blocks and permits the owner to abandon a stale frame after
    /// each transfer. A subsequent frame always installs a new address window.
    ///
    /// # Errors
    ///
    /// Returns transfer or pure-renderer errors without panicking.
    pub fn render_frame<F>(
        &mut self,
        view: &TftView,
        background: TftBackgroundMode,
        mut should_continue: F,
    ) -> Result<FrameResult, TftDisplayError>
    where
        F: FnMut() -> bool,
    {
        self.set_window(0, 0, TFT_WIDTH, TFT_HEIGHT)?;
        self.dc
            .set_high()
            .map_err(|_| TftDisplayError::DataCommandPin)?;
        let mut transfer = [0_u8; TFT_TRANSFER_BYTES];
        for block_y in (0..TFT_HEIGHT).step_by(TFT_ROWS_PER_TRANSFER) {
            let rows = usize::from(TFT_HEIGHT - block_y).min(TFT_ROWS_PER_TRANSFER);
            let bytes = self
                .renderer
                .render_block(view, background, block_y, rows, &mut transfer)
                .map_err(TftDisplayError::Rendering)?;
            self.spi
                .write(&transfer[..bytes])
                .map_err(|_| TftDisplayError::SpiTransfer)?;
            task::do_yield();
            if !should_continue() {
                return Ok(FrameResult::Superseded);
            }
        }
        self.renderer.advance(background);
        Ok(FrameResult::Complete)
    }

    fn initialize(&mut self) -> Result<(), TftDisplayError> {
        self.reset
            .set_high()
            .map_err(|_| TftDisplayError::ResetPin)?;
        FreeRtos::delay_ms(10);
        self.reset
            .set_low()
            .map_err(|_| TftDisplayError::ResetPin)?;
        FreeRtos::delay_ms(20);
        self.reset
            .set_high()
            .map_err(|_| TftDisplayError::ResetPin)?;
        FreeRtos::delay_ms(120);

        self.command(0x01, &[])?;
        FreeRtos::delay_ms(150);
        self.command(0x11, &[])?;
        FreeRtos::delay_ms(120);
        self.command(0x3a, &[0x55])?;
        self.command(0x36, &[0x00])?;
        self.command(0x21, &[])?;
        self.command(0x13, &[])?;
        FreeRtos::delay_ms(10);
        self.command(0x29, &[])?;
        FreeRtos::delay_ms(120);
        Ok(())
    }

    fn command(&mut self, value: u8, data: &[u8]) -> Result<(), TftDisplayError> {
        self.dc
            .set_low()
            .map_err(|_| TftDisplayError::DataCommandPin)?;
        self.spi
            .write(&[value])
            .map_err(|_| TftDisplayError::SpiTransfer)?;
        if !data.is_empty() {
            self.dc
                .set_high()
                .map_err(|_| TftDisplayError::DataCommandPin)?;
            self.spi
                .write(data)
                .map_err(|_| TftDisplayError::SpiTransfer)?;
        }
        Ok(())
    }

    fn set_window(
        &mut self,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    ) -> Result<(), TftDisplayError> {
        let Some(x_end) = x.checked_add(width).and_then(|end| end.checked_sub(1)) else {
            return Err(TftDisplayError::InvalidWindow);
        };
        let Some(y_end) = y.checked_add(height).and_then(|end| end.checked_sub(1)) else {
            return Err(TftDisplayError::InvalidWindow);
        };
        if width == 0 || height == 0 || x_end >= TFT_WIDTH || y_end >= TFT_HEIGHT {
            return Err(TftDisplayError::InvalidWindow);
        }
        self.command(
            0x2a,
            &[(x >> 8) as u8, x as u8, (x_end >> 8) as u8, x_end as u8],
        )?;
        self.command(
            0x2b,
            &[(y >> 8) as u8, y as u8, (y_end >> 8) as u8, y_end as u8],
        )?;
        self.command(0x2c, &[])
    }
}
