use std::{thread, time::Duration};

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
    delay::BLOCK,
    gpio::{Output, PinDriver, Pull},
    i2c::{I2cConfig, I2cDriver},
    peripherals::Peripherals,
    units::KiloHertz,
};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
#[cfg(not(feature = "acceptance-diagnostic"))]
use focus_core::DEFAULT_PRESETS;
use focus_core::{App, Catalog, Effects, InputEvent, Preset, SettingsLoad};
use focus_firmware::{
    buzzer::BuzzerCadence,
    input::EncoderInput,
    journal_adapter::{FlushOutcome as JournalFlushOutcome, OutcomeJournalQueue},
    presentation::{OLED_LAYOUT, OledView, oled_view},
    settings::{FlushOutcome, QueueOutcome, SelectionPersistence, StoredSettings, load_settings},
};
use focus_sync::{JOURNAL_CAPACITY, PersistentJournal, VolatileClock};
use ssd1306::{
    I2CDisplayInterface, Ssd1306,
    mode::DisplayConfig,
    prelude::{DisplayRotation, DisplaySize128x64},
};

use crate::{
    clock::MonotonicClock,
    nvs_journal::{EspEntropy, NvsJournalStore},
    nvs_settings::NvsSettingsStore,
};

const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(1);
const TIME_ADVANCE_INTERVAL_MS: u64 = 10;
const RENDER_RETRY_MS: u64 = 250;
const OLED_ADDRESS: u8 = 0x3c;

#[cfg(not(feature = "acceptance-diagnostic"))]
const RUNTIME_PRESETS: [Preset; 5] = DEFAULT_PRESETS;

/// Short copies of the production presets for an on-device lifecycle check.
///
/// IDs, names, ordering, input handling, rendering, and feedback remain the
/// same as production. Only durations are shortened so completion can be
/// observed without waiting fifteen minutes.
#[cfg(feature = "acceptance-diagnostic")]
const RUNTIME_PRESETS: [Preset; 5] = [
    Preset::new("deep-work", "Deep Work", 8_000),
    Preset::new("focus", "Focus", 8_000),
    Preset::new("pomodoro", "Pomodoro", 8_000),
    Preset::new("reading", "Reading", 8_000),
    Preset::new("quick-sprint", "Quick Sprint", 8_000),
];

/// Owns application state and serializes semantic encoder and clock events.
pub fn run() -> ! {
    let peripherals = Peripherals::take().expect("ESP32 peripherals must be available once");

    let s1 = PinDriver::input(peripherals.pins.gpio0, Pull::Up)
        .expect("failed to configure EC11 S1 on GPIO0");
    let s2 = PinDriver::input(peripherals.pins.gpio4, Pull::Up)
        .expect("failed to configure EC11 S2 on GPIO4");
    let key = PinDriver::input(peripherals.pins.gpio5, Pull::Up)
        .expect("failed to configure EC11 KEY on GPIO5");
    let mut buzzer_output = match PinDriver::output(peripherals.pins.gpio1) {
        Ok(mut output) => match output.set_low() {
            Ok(()) => Some(output),
            Err(error) => {
                log::error!(
                    "failed to command buzzer GPIO1 off at startup: {error:?}; continuing without audible feedback"
                );
                None
            }
        },
        Err(error) => {
            log::error!(
                "failed to configure buzzer GPIO1: {error:?}; continuing without audible feedback"
            );
            None
        }
    };
    let mut buzzer_cadence = BuzzerCadence::new();

    let boot_s1 = s1.is_high();
    let boot_s2 = s2.is_high();
    let boot_key = key.is_high();
    // The delivered module's clockwise sequence is S1-leading. Feeding S2 as
    // generic phase A preserves clockwise=RotateRight.
    let mut input = EncoderInput::new(boot_s2, boot_s1, !boot_key);

    let i2c_config = I2cConfig::new()
        .baudrate(KiloHertz(100).into())
        .sda_enable_pullup(true)
        .scl_enable_pullup(true);
    let mut i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio7,
        peripherals.pins.gpio6,
        &i2c_config,
    )
    .expect("failed to configure OLED I2C on GPIO6/GPIO7");
    i2c.write(OLED_ADDRESS, &[0x00, 0xae], BLOCK)
        .expect("OLED must ACK at the bench-verified address 0x3C");

    let interface = I2CDisplayInterface::new_custom_address(i2c, OLED_ADDRESS);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().expect("SSD1306 initialization must succeed");

    let catalog = Catalog::new(&RUNTIME_PRESETS, 2).expect("firmware catalog must be valid");
    let nvs_partition = match EspDefaultNvsPartition::take() {
        Ok(partition) => Some(partition),
        Err(error) => {
            log::error!(
                "default NVS partition unavailable: {error:?}; continuing without settings or journal persistence"
            );
            None
        }
    };
    let mut settings_store = match nvs_partition.as_ref() {
        Some(partition) => match NvsSettingsStore::open(partition.clone()) {
            Ok(store) => Some(store),
            Err(error) => {
                log::error!(
                    "settings NVS unavailable: {error:?}; booting with the default and continuing without selection persistence"
                );
                None
            }
        },
        None => None,
    };
    let mut entropy = EspEntropy;
    let mut session_journal = match nvs_partition.as_ref() {
        Some(partition) => match NvsJournalStore::open(partition.clone()) {
            Ok(store) => {
                match PersistentJournal::<_, JOURNAL_CAPACITY>::initialize(store, &mut entropy) {
                    Ok((journal, report)) => {
                        log::info!(
                            "session journal ready: device={:02x?} epoch={:02x?} bounds={:?} health={:?} init={report:?}",
                            journal.journal().device_id(),
                            journal.journal().epoch(),
                            journal.journal().bounds(),
                            journal.journal().health(),
                        );
                        Some(journal)
                    }
                    Err(error) => {
                        log::error!(
                            "session journal initialization failed: {error:?}; timer remains usable without synchronization durability"
                        );
                        None
                    }
                }
            }
            Err(error) => {
                log::error!(
                    "focus_sync NVS namespace unavailable: {error:?}; timer remains usable without synchronization durability"
                );
                None
            }
        },
        None => None,
    };
    let mut settings_read_failed = settings_store.is_none();
    let loaded_settings = settings_store
        .as_mut()
        .and_then(|store| match load_settings(store) {
            Ok(Some(settings)) => {
                log::info!("settings record loaded from NVS");
                Some(settings)
            }
            Ok(None) => {
                log::info!("settings record is absent; using the firmware default");
                None
            }
            Err(error) => {
                settings_read_failed = true;
                log::warn!("settings read/decode failed: {error:?}; using the firmware default");
                None
            }
        });
    let boot_settings = if settings_read_failed {
        SettingsLoad::Corrupt
    } else {
        loaded_settings
            .as_ref()
            .map_or(SettingsLoad::Missing, StoredSettings::as_core_load)
    };
    let mut selection_persistence = SelectionPersistence::new(
        loaded_settings
            .as_ref()
            .and_then(StoredSettings::current_persisted_id),
    );
    let (mut app, boot_effects) = App::boot(catalog, boot_settings);
    let clock = MonotonicClock::new();
    let wall_clock = VolatileClock::new();
    let mut outcome_queue = OutcomeJournalQueue::new();
    let mut next_time_advance_ms = 0;
    let mut next_render_attempt_ms = 0;
    let mut render_pending = boot_effects.render;
    let mut rendered_second = None;

    if let Some(diagnostic) = boot_effects.diagnostic {
        log::warn!("core boot diagnostic: {diagnostic:?}");
    }

    log::info!(
        "integrated runtime ready: OLED=0x{OLED_ADDRESS:02X} GPIO6/GPIO7; EC11 S1=GPIO0 S2=GPIO4 KEY=GPIO5; active buzzer=GPIO1 through 330 ohm; boot levels S1={boot_s1} S2={boot_s2} KEY={boot_key}"
    );
    #[cfg(feature = "acceptance-diagnostic")]
    log::warn!("ACCEPTANCE DIAGNOSTIC: all five presets are temporarily shortened to 8 seconds");

    loop {
        let now_ms = clock.now_ms();
        let events = input.sample(now_ms, s2.is_high(), s1.is_high(), key.is_low());
        if let Some(event) = events.rotation {
            process_event(
                &mut app,
                now_ms,
                event,
                &mut render_pending,
                &mut buzzer_cadence,
                &mut buzzer_output,
                &mut selection_persistence,
                &mut outcome_queue,
                wall_clock,
            );
        }
        if let Some(event) = events.button {
            process_event(
                &mut app,
                now_ms,
                event,
                &mut render_pending,
                &mut buzzer_cadence,
                &mut buzzer_output,
                &mut selection_persistence,
                &mut outcome_queue,
                wall_clock,
            );
        }

        if now_ms >= next_time_advance_ms {
            process_event(
                &mut app,
                now_ms,
                InputEvent::TimeAdvanced,
                &mut render_pending,
                &mut buzzer_cadence,
                &mut buzzer_output,
                &mut selection_persistence,
                &mut outcome_queue,
                wall_clock,
            );
            next_time_advance_ms = now_ms.saturating_add(TIME_ADVANCE_INTERVAL_MS);
        }

        if let Some(on) = buzzer_cadence.update(now_ms) {
            set_buzzer_output(&mut buzzer_output, on);
        }

        if let Some(store) = settings_store.as_mut() {
            match selection_persistence.flush_due(now_ms, store) {
                Ok(FlushOutcome::NotDue) => {}
                Ok(FlushOutcome::Saved(id)) => {
                    log::info!("saved selected preset to NVS: {}", id.as_str());
                }
                Err(error) => {
                    log::error!(
                        "settings write failed: {error:?}; keeping the in-memory selection and waiting for a later selection change"
                    );
                }
            }
        }

        if let Some(journal) = session_journal.as_mut() {
            match outcome_queue.flush_due(now_ms, journal) {
                JournalFlushOutcome::Idle | JournalFlushOutcome::Waiting => {}
                JournalFlushOutcome::Saved { sequence } => {
                    log::info!("session journal append committed: sequence={sequence}");
                }
                JournalFlushOutcome::Failed { error } => {
                    log::error!(
                        "session journal append failed: {error:?}; retaining one pending outcome for bounded retry"
                    );
                }
            }
        }

        let snapshot = app.snapshot(now_ms);
        let visible_second = visible_second(snapshot.remaining_ms);
        if rendered_second != Some(visible_second) {
            render_pending = true;
        }

        if render_pending && now_ms >= next_render_attempt_ms {
            let view = oled_view(snapshot);
            display.clear_buffer();
            draw_view(&mut display, &view).expect("drawing into the OLED buffer must succeed");
            match display.flush() {
                Ok(()) => {
                    log::info!(
                        "OLED render: state={} preset={} time={}",
                        view.state_label,
                        view.preset_name,
                        view.time.as_str()
                    );
                    render_pending = false;
                    rendered_second = Some(visible_second);
                }
                Err(error) => {
                    log::error!(
                        "OLED render failed: {error:?}; timer state retained, retrying newest snapshot"
                    );
                    next_render_attempt_ms = now_ms.saturating_add(RENDER_RETRY_MS);
                }
            }
        }

        thread::sleep(INPUT_POLL_INTERVAL);
    }
}

fn process_event(
    app: &mut App,
    now_ms: u64,
    event: InputEvent,
    render_pending: &mut bool,
    buzzer_cadence: &mut BuzzerCadence,
    buzzer_output: &mut Option<PinDriver<'_, Output>>,
    selection_persistence: &mut SelectionPersistence,
    outcome_queue: &mut OutcomeJournalQueue,
    wall_clock: VolatileClock,
) {
    let before = app.snapshot(now_ms).state;
    let effects = app.handle(now_ms, event);
    let after = app.snapshot(now_ms).state;
    match outcome_queue.observe(before, after, now_ms, effects.outcome, wall_clock) {
        Ok(focus_firmware::journal_adapter::ObserveOutcome::NoRecord) => {}
        Ok(focus_firmware::journal_adapter::ObserveOutcome::Queued) => {
            log::info!("committed session outcome queued for journal append");
        }
        Err(error) => {
            log::error!(
                "committed session outcome could not enter the bounded journal queue: {error:?}; timer transition remains committed"
            );
        }
    }
    if event != InputEvent::TimeAdvanced {
        log::info!("input event: {event:?}");
    }
    observe_effects(
        effects,
        render_pending,
        now_ms,
        buzzer_cadence,
        buzzer_output,
        selection_persistence,
    );
}

fn observe_effects(
    effects: Effects,
    render_pending: &mut bool,
    now_ms: u64,
    buzzer_cadence: &mut BuzzerCadence,
    buzzer_output: &mut Option<PinDriver<'_, Output>>,
    selection_persistence: &mut SelectionPersistence,
) {
    *render_pending |= effects.render;
    if let Some(feedback) = effects.feedback {
        let on = buzzer_cadence.trigger(now_ms, feedback);
        set_buzzer_output(buzzer_output, on);
        log::info!("buzzer feedback started: {feedback:?}");
    }
    if let Some(preset) = effects.persist_selection {
        match selection_persistence.selection_changed(now_ms, preset) {
            Ok(QueueOutcome::Scheduled) => {
                log::info!(
                    "selection save scheduled after quiet time: {}",
                    preset.as_str()
                );
            }
            Ok(QueueOutcome::AlreadyPersisted) => {
                log::info!("selection already persisted: {}", preset.as_str());
            }
            Err(error) => {
                log::error!(
                    "selection cannot be encoded for persistence: {error:?}; in-memory selection remains active"
                );
            }
        }
    }
    if let Some(diagnostic) = effects.diagnostic {
        log::warn!("core diagnostic: {diagnostic:?}");
    }
}

fn set_buzzer_output(output: &mut Option<PinDriver<'_, Output>>, on: bool) {
    let result = output.as_mut().map(|output| {
        if on {
            output.set_high()
        } else {
            output.set_low()
        }
    });

    if let Some(Err(error)) = result {
        log::error!(
            "buzzer GPIO update failed: {error:?}; disabling audible feedback while timer state continues"
        );
        *output = None;
    }
}

fn visible_second(remaining_ms: u64) -> u64 {
    remaining_ms / 1_000 + u64::from(remaining_ms % 1_000 != 0)
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
        view.preset_name,
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
