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
    protocol_projection::{session_page_response, status_response},
    protocol_session::{ProtocolAction, ProtocolSession},
    settings::{FlushOutcome, QueueOutcome, SelectionPersistence, StoredSettings, load_settings},
};
use focus_protocol::{
    Capability, ClockAnchorResponse, ErrorCode, ErrorResponse, HelloResponse,
    MAX_LOGICAL_MESSAGE_BYTES, ProtocolVersion, Response, ResponseEnvelope, decode_request,
    encode_response,
};
use focus_sync::{
    JOURNAL_CAPACITY, JournalStatus, PersistentJournal, VolatileClock, project_status,
};
use ssd1306::{
    I2CDisplayInterface, Ssd1306,
    mode::DisplayConfig,
    prelude::{DisplayRotation, DisplaySize128x64},
};

use crate::{
    ble_radio::{BleRadio, NotificationProgress},
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
// Hardware acquisition, optional adapter setup, and the one authoritative
// event loop are intentionally visible together for ownership auditing.
#[allow(clippy::too_many_lines)]
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
    let mut wall_clock = VolatileClock::new();
    let mut protocol_session = session_journal
        .as_ref()
        .map(|journal| ProtocolSession::new(protocol_hello(journal.journal().device_id())));
    let mut ble_radio = if protocol_session.is_some() {
        match BleRadio::start() {
            Ok(radio) => Some(radio),
            Err(error) => {
                log::error!(
                    "BLE service initialization failed: {error:?}; continuing as a complete offline timer"
                );
                None
            }
        }
    } else {
        log::warn!(
            "BLE service disabled because no stable journal identity is available; offline timer remains active"
        );
        None
    };
    let mut observed_ble_generation = ble_radio
        .as_ref()
        .map_or(0, |radio| radio.connection_snapshot().generation);
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

        if let (Some(radio), Some(session)) = (ble_radio.as_mut(), protocol_session.as_mut()) {
            let connection = radio.connection_snapshot();
            if connection.generation != observed_ble_generation {
                observed_ble_generation = connection.generation;
                session.reset();
                log::info!(
                    "BLE protocol session reset for connection generation={observed_ble_generation} connected={} subscribed={} mtu={}",
                    connection.connected,
                    connection.subscribed,
                    connection.mtu
                );
            }

            if let Some(message) = radio.take_request(now_ms) {
                if message.connection_generation == observed_ble_generation {
                    process_protocol_message(
                        message.as_slice(),
                        now_ms,
                        session,
                        radio,
                        &app,
                        session_journal.as_ref(),
                        &mut wall_clock,
                    );
                } else {
                    log::warn!(
                        "discarded stale BLE request from generation={} current={observed_ble_generation}",
                        message.connection_generation
                    );
                }
            }

            match radio.poll_notification() {
                Ok(NotificationProgress::Idle | NotificationProgress::Sent) => {}
                Ok(NotificationProgress::Complete) => {
                    log::debug!("BLE logical response notification transfer complete");
                }
                Ok(NotificationProgress::DroppedConnection) => {
                    log::warn!("BLE logical response dropped after connection lifecycle changed");
                }
                Err(error) => {
                    log::warn!(
                        "BLE notification failed; client may retry the read-only request: {error:?}"
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

fn protocol_hello(device_id: [u8; 16]) -> HelloResponse {
    let mut capabilities = heapless::Vec::new();
    capabilities
        .push(Capability::ReadStatus)
        .expect("three capabilities fit the protocol registry");
    capabilities
        .push(Capability::ReadSessionPages)
        .expect("three capabilities fit the protocol registry");
    capabilities
        .push(Capability::SetClockAnchor)
        .expect("three capabilities fit the protocol registry");
    HelloResponse {
        device_id,
        product_name: "FocusTimer"
            .try_into()
            .expect("product name fits the protocol registry"),
        firmware_version: env!("CARGO_PKG_VERSION")
            .try_into()
            .expect("firmware version fits the protocol registry"),
        supported_version: ProtocolVersion::CURRENT,
        capabilities,
    }
}

// Logical request handling remains in the owner loop and never runs inside a
// `NimBLE` callback; the explicit inputs make that boundary reviewable.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn process_protocol_message(
    bytes: &[u8],
    now_ms: u64,
    session: &mut ProtocolSession,
    radio: &mut BleRadio,
    app: &App,
    journal: Option<&PersistentJournal<NvsJournalStore, JOURNAL_CAPACITY>>,
    wall_clock: &mut VolatileClock,
) {
    let request = match decode_request(bytes) {
        Ok(request) => request,
        Err(error) => {
            log::warn!(
                "BLE logical request decode failed: bytes={} error={error:?}; timer and journal were not mutated",
                bytes.len()
            );
            radio.abandon_response();
            return;
        }
    };
    let action = session.handle(&request);
    let response = match action {
        ProtocolAction::Respond(response) => response,
        ProtocolAction::ReadStatus {
            request_id,
            version,
        } => match journal {
            Some(journal) => {
                let model = journal.journal();
                let (oldest_sequence, latest_sequence) = model.bounds();
                let status = project_status(
                    app,
                    now_ms,
                    JournalStatus {
                        epoch: model.epoch(),
                        oldest_sequence,
                        latest_sequence,
                        health: model.health(),
                    },
                    *wall_clock,
                );
                match status_response(status) {
                    Ok(status) => ResponseEnvelope {
                        version,
                        request_id,
                        response: Response::Status(status),
                    },
                    Err(error) => {
                        log::error!("BLE status projection failed: {error:?}");
                        protocol_error(request_id, version, ErrorCode::InternalError, Some(3), None)
                    }
                }
            }
            None => protocol_error(
                request_id,
                version,
                ErrorCode::JournalUnavailable,
                Some(3),
                None,
            ),
        },
        ProtocolAction::ReadSessionPage {
            request_id,
            version,
            page,
        } => match journal {
            Some(journal) => match journal.page::<8>(
                page.journal_epoch,
                page.after_sequence,
                usize::from(page.limit),
            ) {
                Ok(page) => match session_page_response(&page) {
                    Ok(page) => ResponseEnvelope {
                        version,
                        request_id,
                        response: Response::SessionPage(page),
                    },
                    Err(error) => {
                        log::error!("BLE journal page projection failed: {error:?}");
                        protocol_error(request_id, version, ErrorCode::InternalError, Some(5), None)
                    }
                },
                Err(error) => {
                    log::warn!("BLE journal page request rejected: {error:?}");
                    protocol_error(
                        request_id,
                        version,
                        ErrorCode::InvalidField,
                        Some(5),
                        Some(2),
                    )
                }
            },
            None => protocol_error(
                request_id,
                version,
                ErrorCode::JournalUnavailable,
                Some(5),
                None,
            ),
        },
        ProtocolAction::SetClockAnchor {
            request_id,
            version,
            utc_ms,
        } => match wall_clock.set_anchor(utc_ms, now_ms) {
            Ok(()) => ResponseEnvelope {
                version,
                request_id,
                response: Response::ClockAnchor(ClockAnchorResponse {
                    accepted_utc_ms: utc_ms,
                    device_monotonic_ms_at_receipt: now_ms,
                }),
            },
            Err(error) => {
                log::warn!("BLE clock anchor rejected: {error:?}");
                protocol_error(
                    request_id,
                    version,
                    ErrorCode::InvalidField,
                    Some(7),
                    Some(0),
                )
            }
        },
    };

    let mut encoded = [0; MAX_LOGICAL_MESSAGE_BYTES];
    let length = match encode_response(&response, &mut encoded) {
        Ok(length) => length,
        Err(error) => {
            log::error!("BLE logical response encoding failed: {error:?}");
            radio.abandon_response();
            return;
        }
    };
    if let Err(error) = radio.queue_response(&encoded[..length]) {
        log::warn!(
            "BLE logical response could not enter bounded outbox: request_id={} error={error:?}",
            response.request_id
        );
        radio.abandon_response();
    }
}

fn protocol_error(
    request_id: u32,
    version: ProtocolVersion,
    code: ErrorCode,
    failed_message_kind: Option<u64>,
    field_id: Option<u64>,
) -> ResponseEnvelope {
    ResponseEnvelope {
        version,
        request_id,
        response: Response::Error(ErrorResponse {
            code,
            failed_message_kind,
            field_id,
            supported_version: None,
        }),
    }
}

// These are independent best-effort adapters observing one committed core
// transition; grouping them avoids introducing another mutable state owner.
#[allow(clippy::too_many_arguments)]
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
