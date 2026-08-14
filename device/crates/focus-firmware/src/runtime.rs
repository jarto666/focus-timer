use std::{boxed::Box, thread, time::Duration};

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
use embedded_hal::i2c::{ErrorType, I2c, Operation};
use esp_idf_svc::hal::{
    delay::TickType,
    gpio::{Output, PinDriver, Pull},
    i2c::{I2cConfig, I2cDriver, I2cError},
    peripherals::Peripherals,
    units::KiloHertz,
};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use focus_core::{
    App, Catalog, CatalogConfirmationAction, CatalogStageError, CatalogUpdateCoordinator, Effects,
    InputEvent, Preset, SettingsLoad, ViewState, default_presets,
};
use focus_firmware::{
    buzzer::BuzzerCadence,
    input::EncoderInput,
    journal_adapter::{FlushOutcome as JournalFlushOutcome, OutcomeJournalQueue},
    presentation::{OLED_LAYOUT, OledView, catalog_confirmation_view, oled_view},
    preset_storage::{commit_catalog, load_catalog},
    protocol_projection::{session_page_response, status_response_with_order},
    protocol_session::{ProtocolAction, ProtocolSession},
    settings::{FlushOutcome, QueueOutcome, SelectionPersistence, StoredSettings, load_settings},
};
use focus_protocol::{
    Capability, CatalogEntry, CatalogResult, ClockAnchorResponse, DeviceEvent, ErrorCode,
    ErrorResponse, EventEnvelope, HelloResponse, MAX_LOGICAL_MESSAGE_BYTES, PresetCatalogResponse,
    PresetCatalogResultEvent, PresetSnapshot, ProposePresetCatalogResponse, ProtocolVersion,
    Response, ResponseEnvelope, decode_request, encode_event, encode_response,
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
    nvs_presets::NvsPresetCatalogStore,
    nvs_settings::NvsSettingsStore,
};

const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(1);
const TIME_ADVANCE_INTERVAL_MS: u64 = 10;
const RENDER_RETRY_MS: u64 = 250;
const OLED_ADDRESS: u8 = 0x3c;
const I2C_TRANSACTION_TIMEOUT: u32 = TickType::new_millis(100).ticks();
#[cfg(feature = "ble-fault-diagnostic")]
const FAULT_DIAGNOSTIC_NOTIFICATION_INTERVAL_MS: u64 = 100;

type DeviceJournal = PersistentJournal<NvsJournalStore, JOURNAL_CAPACITY>;

/// Gives the display driver a finite deadline instead of the HAL's default
/// `BLOCK` sentinel. A loose OLED wire must not stall the authoritative timer
/// task until the interrupt watchdog resets the whole device.
struct BoundedI2c<'d> {
    driver: I2cDriver<'d>,
}

impl<'d> BoundedI2c<'d> {
    fn new(driver: I2cDriver<'d>) -> Self {
        Self { driver }
    }
}

impl ErrorType for BoundedI2c<'_> {
    type Error = I2cError;
}

impl I2c for BoundedI2c<'_> {
    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.driver
            .read(address, buffer, I2C_TRANSACTION_TIMEOUT)
            .map_err(Into::into)
    }

    fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.driver
            .write(address, bytes, I2C_TRANSACTION_TIMEOUT)
            .map_err(Into::into)
    }

    fn write_read(
        &mut self,
        address: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        self.driver
            .write_read(address, bytes, buffer, I2C_TRANSACTION_TIMEOUT)
            .map_err(Into::into)
    }

    fn transaction(
        &mut self,
        address: u8,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        self.driver
            .transaction(address, operations, I2C_TRANSACTION_TIMEOUT)
            .map_err(Into::into)
    }
}

/// Builds the large fixed-capacity journal away from the already sizeable
/// authoritative-loop frame, then keeps it on the heap for the device's
/// lifetime. This avoids stacking the 64-record journal and its 28 KiB NVS
/// reconstruction workspace on top of each other.
#[inline(never)]
fn initialize_session_journal(
    store: NvsJournalStore,
    entropy: &mut EspEntropy,
) -> Option<Box<DeviceJournal>> {
    match PersistentJournal::<_, JOURNAL_CAPACITY>::initialize(store, entropy) {
        Ok((journal, report)) => {
            let journal = Box::new(journal);
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

/// Short copies of the production presets for an on-device lifecycle check.
///
/// IDs, names, ordering, input handling, rendering, and feedback remain the
/// same as production. Only durations are shortened so completion can be
/// observed without waiting fifteen minutes.
fn runtime_built_ins() -> heapless::Vec<Preset, 13> {
    #[cfg(not(any(
        feature = "acceptance-diagnostic",
        feature = "radio-failure-diagnostic"
    )))]
    {
        return default_presets();
    }
    #[cfg(any(
        feature = "acceptance-diagnostic",
        feature = "radio-failure-diagnostic"
    ))]
    {
        let mut presets = heapless::Vec::new();
        for (id, name) in [
            ("deep-work", "Deep Work"),
            ("focus", "Focus"),
            ("pomodoro", "Pomodoro"),
            ("reading", "Reading"),
            ("quick-sprint", "Quick Sprint"),
        ] {
            presets
                .push(Preset::built_in(id, name, 8_000))
                .expect("five diagnostics presets fit");
        }
        presets
    }
}

/// Owns application state and serializes semantic encoder and clock events.
// Hardware acquisition, optional adapter setup, and the one authoritative
// event loop are intentionally visible together for ownership auditing.
#[allow(clippy::too_many_lines)]
pub fn run() -> ! {
    let peripherals = Peripherals::take().expect("ESP32 peripherals must be available once");
    log::info!("runtime stage: peripherals acquired");

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
    log::info!("runtime stage: encoder and buzzer GPIO configured");

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
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio7,
        peripherals.pins.gpio6,
        &i2c_config,
    );
    let mut display = match i2c {
        Ok(mut i2c) => {
            log::info!("runtime stage: probing OLED at 0x{OLED_ADDRESS:02X}");
            match i2c.write(OLED_ADDRESS, &[0x00, 0xae], I2C_TRANSACTION_TIMEOUT) {
                Ok(()) => {
                    let interface =
                        I2CDisplayInterface::new_custom_address(BoundedI2c::new(i2c), OLED_ADDRESS);
                    let mut candidate =
                        Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
                            .into_buffered_graphics_mode();
                    match candidate.init() {
                        Ok(()) => {
                            log::info!("runtime stage: OLED initialized");
                            Some(candidate)
                        }
                        Err(error) => {
                            log::error!(
                                "SSD1306 initialization failed: {error:?}; continuing without a display"
                            );
                            None
                        }
                    }
                }
                Err(error) => {
                    log::error!(
                        "OLED did not ACK within 100 ms: {error:?}; continuing without a display"
                    );
                    None
                }
            }
        }
        Err(error) => {
            log::error!(
                "failed to configure OLED I2C on GPIO6/GPIO7: {error:?}; continuing without a display"
            );
            None
        }
    };

    log::info!("runtime stage: opening persistent stores");
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
    let mut preset_store = match nvs_partition.as_ref() {
        Some(partition) => match NvsPresetCatalogStore::open(partition.clone()) {
            Ok(store) => Some(store),
            Err(error) => {
                log::error!(
                    "preset catalog NVS unavailable: {error:?}; continuing with built-ins only"
                );
                None
            }
        },
        None => None,
    };
    let loaded_catalog = preset_store
        .as_mut()
        .and_then(|store| match load_catalog(store) {
            Ok(catalog) => {
                if catalog.degraded {
                    log::warn!(
                        "preset catalog recovered in degraded mode at revision={}",
                        catalog.revision
                    );
                }
                Some(catalog)
            }
            Err(error) => {
                log::error!(
                    "preset catalog read failed: {error:?}; continuing with built-ins only"
                );
                None
            }
        });
    let mut runtime_presets = runtime_built_ins();
    if let Some(stored) = loaded_catalog.as_ref() {
        for preset in &stored.custom_entries {
            runtime_presets
                .push(preset.clone())
                .expect("persisted catalog validation enforces total capacity");
        }
    }
    let catalog = Catalog::combined(runtime_presets.as_slice())
        .expect("firmware and recovered catalog must be valid");
    let mut catalog_revision = loaded_catalog
        .as_ref()
        .map_or(0, |catalog| catalog.revision);
    let mut entropy = EspEntropy;
    let mut session_journal = match nvs_partition.as_ref() {
        Some(partition) => match NvsJournalStore::open(partition.clone()) {
            Ok(store) => initialize_session_journal(store, &mut entropy),
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
    let mut status_epoch = [0_u8; 8];
    if let Err(error) = getrandom::fill(&mut status_epoch) {
        log::warn!("status epoch entropy failed: {error:?}; deriving it from device identity");
        if let Some(journal) = session_journal.as_ref() {
            status_epoch.copy_from_slice(&journal.journal().device_id()[..8]);
        } else {
            status_epoch = clock.now_ms().to_be_bytes();
        }
    }
    let mut status_revision = 1_u64;
    let mut live_status_pending = true;
    let mut pending_catalog_result: Option<PresetCatalogResultEvent> = None;
    let mut catalog_updates = CatalogUpdateCoordinator::new();
    let mut protocol_session = session_journal
        .as_ref()
        .map(|journal| ProtocolSession::new(protocol_hello(journal.journal().device_id())));
    let mut ble_radio: Option<BleRadio> = if protocol_session.is_some() {
        #[cfg(feature = "radio-failure-diagnostic")]
        {
            log::error!(
                "RADIO FAILURE DIAGNOSTIC: injected BLE initialization failure; continuing as a complete offline timer"
            );
            None
        }
        #[cfg(not(feature = "radio-failure-diagnostic"))]
        {
            match BleRadio::start() {
                Ok(radio) => Some(radio),
                Err(error) => {
                    log::error!(
                        "BLE service initialization failed: {error:?}; continuing as a complete offline timer"
                    );
                    None
                }
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
    let mut next_live_status_ms = 0;
    let mut next_render_attempt_ms = 0;
    let mut render_pending = boot_effects.render;
    let mut rendered_second = None;
    #[cfg(feature = "ble-fault-diagnostic")]
    let mut next_notification_poll_ms = 0;

    if let Some(diagnostic) = boot_effects.diagnostic {
        log::warn!("core boot diagnostic: {diagnostic:?}");
    }

    log::info!(
        "integrated runtime ready: OLED=0x{OLED_ADDRESS:02X} GPIO6/GPIO7; EC11 S1=GPIO0 S2=GPIO4 KEY=GPIO5; active buzzer=GPIO1 through 330 ohm; boot levels S1={boot_s1} S2={boot_s2} KEY={boot_key}"
    );
    log_resource_snapshot("idle-ready");
    #[cfg(feature = "acceptance-diagnostic")]
    log::warn!("ACCEPTANCE DIAGNOSTIC: all five presets are temporarily shortened to 8 seconds");
    #[cfg(feature = "ble-fault-diagnostic")]
    log::warn!(
        "BLE FAULT DIAGNOSTIC: response fragments are intentionally spaced by {FAULT_DIAGNOSTIC_NOTIFICATION_INTERVAL_MS} ms"
    );
    #[cfg(feature = "radio-failure-diagnostic")]
    log::warn!(
        "RADIO FAILURE DIAGNOSTIC: BLE is intentionally absent and all five presets are temporarily shortened to 8 seconds"
    );

    loop {
        let now_ms = clock.now_ms();
        let events = input.sample(now_ms, s2.is_high(), s1.is_high(), key.is_low());
        if let Some(event) = events.rotation {
            if catalog_updates.pending_proposal_id().is_none()
                && process_event(
                    &mut app,
                    now_ms,
                    event,
                    &mut render_pending,
                    &mut buzzer_cadence,
                    &mut buzzer_output,
                    &mut selection_persistence,
                    &mut outcome_queue,
                    wall_clock,
                )
            {
                mark_live_status(&mut status_revision, &mut live_status_pending);
            }
        }
        if let Some(event) = events.button {
            if catalog_updates.pending_proposal_id().is_some() {
                let action = catalog_updates.handle_input(now_ms, event);
                let resolution = resolve_catalog_action(
                    action,
                    &mut app,
                    preset_store.as_mut(),
                    &mut catalog_revision,
                );
                if let Some(effects) = resolution.effects {
                    observe_effects(
                        effects,
                        &mut render_pending,
                        now_ms,
                        &mut buzzer_cadence,
                        &mut buzzer_output,
                        &mut selection_persistence,
                    );
                }
                if let Some(result) = resolution.result {
                    pending_catalog_result = Some(result);
                    render_pending = true;
                }
                if resolution.catalog_changed {
                    mark_live_status(&mut status_revision, &mut live_status_pending);
                }
            } else if process_event(
                &mut app,
                now_ms,
                event,
                &mut render_pending,
                &mut buzzer_cadence,
                &mut buzzer_output,
                &mut selection_persistence,
                &mut outcome_queue,
                wall_clock,
            ) {
                mark_live_status(&mut status_revision, &mut live_status_pending);
            }
        }

        if now_ms >= next_time_advance_ms {
            if process_event(
                &mut app,
                now_ms,
                InputEvent::TimeAdvanced,
                &mut render_pending,
                &mut buzzer_cadence,
                &mut buzzer_output,
                &mut selection_persistence,
                &mut outcome_queue,
                wall_clock,
            ) {
                mark_live_status(&mut status_revision, &mut live_status_pending);
            }
            next_time_advance_ms = now_ms.saturating_add(TIME_ADVANCE_INTERVAL_MS);
        }

        if let Some(action) = catalog_updates.expire(now_ms) {
            let resolution = resolve_catalog_action(
                action,
                &mut app,
                preset_store.as_mut(),
                &mut catalog_revision,
            );
            pending_catalog_result = resolution.result;
            render_pending = true;
        }

        if app.snapshot(now_ms).state == ViewState::Running && now_ms >= next_live_status_ms {
            mark_live_status(&mut status_revision, &mut live_status_pending);
            next_live_status_ms = now_ms.saturating_add(1_000);
        } else if app.snapshot(now_ms).state != ViewState::Running {
            next_live_status_ms = now_ms.saturating_add(1_000);
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

        if let Some(journal) = session_journal.as_deref_mut() {
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
                if !connection.connected {
                    let resolution = resolve_catalog_action(
                        catalog_updates.cancel(),
                        &mut app,
                        preset_store.as_mut(),
                        &mut catalog_revision,
                    );
                    if resolution.result.is_some() {
                        pending_catalog_result = resolution.result;
                        render_pending = true;
                    }
                }
                log::info!(
                    "BLE protocol session reset for connection generation={observed_ble_generation} connected={} subscribed={} event_subscribed={} mtu={}",
                    connection.connected,
                    connection.subscribed,
                    connection.event_subscribed,
                    connection.mtu
                );
                log_resource_snapshot(if connection.connected {
                    "connected"
                } else {
                    "disconnected"
                });
            }

            if let Some(message) = radio.take_request(now_ms) {
                if message.connection_generation == observed_ble_generation {
                    process_protocol_message(
                        message.as_slice(),
                        message.transfer_id,
                        now_ms,
                        session,
                        radio,
                        &app,
                        &mut catalog_updates,
                        catalog_revision,
                        &mut pending_catalog_result,
                        session_journal.as_deref(),
                        &mut wall_clock,
                        status_epoch,
                        status_revision,
                    );
                    if catalog_updates.pending_proposal_id().is_some() {
                        render_pending = true;
                    }
                } else {
                    log::warn!(
                        "discarded stale BLE request from generation={} current={observed_ble_generation}",
                        message.connection_generation
                    );
                }
            }

            let live_events_negotiated = session
                .negotiated_version()
                .is_some_and(|version| version.minor >= 1);
            if connection.event_subscribed && live_events_negotiated && !radio.event_outbound_busy()
            {
                if let Some(result) = pending_catalog_result {
                    if queue_device_event(radio, DeviceEvent::PresetCatalogResult(result)) {
                        pending_catalog_result = None;
                    }
                } else if live_status_pending {
                    if let Some(status) = ordered_status(
                        &app,
                        now_ms,
                        session_journal.as_deref(),
                        wall_clock,
                        status_epoch,
                        status_revision,
                    ) && queue_device_event(radio, DeviceEvent::LiveStatus(status))
                    {
                        live_status_pending = false;
                    }
                }
            }

            let notification_poll_due = {
                #[cfg(feature = "ble-fault-diagnostic")]
                {
                    now_ms >= next_notification_poll_ms
                }
                #[cfg(not(feature = "ble-fault-diagnostic"))]
                {
                    true
                }
            };
            if notification_poll_due {
                match radio.poll_notification() {
                    Ok(NotificationProgress::Idle) => {}
                    Ok(NotificationProgress::Sent) => {
                        #[cfg(feature = "ble-fault-diagnostic")]
                        {
                            next_notification_poll_ms =
                                now_ms.saturating_add(FAULT_DIAGNOSTIC_NOTIFICATION_INTERVAL_MS);
                        }
                    }
                    Ok(NotificationProgress::Complete) => {
                        log::debug!("BLE logical response notification transfer complete");
                        log_resource_snapshot("transfer-complete");
                    }
                    Ok(NotificationProgress::DroppedConnection) => {
                        log::warn!(
                            "BLE logical response dropped after connection lifecycle changed"
                        );
                    }
                    Err(error) => {
                        log::warn!(
                            "BLE notification failed; client may retry the read-only request: {error:?}"
                        );
                    }
                }
                match radio.poll_event_notification() {
                    Ok(NotificationProgress::Idle | NotificationProgress::Sent) => {}
                    Ok(NotificationProgress::Complete) => {
                        log::debug!("BLE logical event notification transfer complete");
                    }
                    Ok(NotificationProgress::DroppedConnection) => {
                        log::warn!("BLE event dropped after connection lifecycle changed");
                    }
                    Err(error) => {
                        log::warn!(
                            "BLE event notification failed; latest state remains recoverable by GetStatus: {error:?}"
                        );
                        live_status_pending = true;
                    }
                }
            }
        }

        let snapshot = app.snapshot(now_ms);
        let visible_second = visible_second(snapshot.remaining_ms);
        if rendered_second != Some(visible_second) {
            render_pending = true;
        }

        if render_pending && now_ms >= next_render_attempt_ms {
            let view = catalog_updates
                .pending_entry_count()
                .map_or_else(|| oled_view(snapshot), catalog_confirmation_view);
            if let Some(display) = display.as_mut() {
                display.clear_buffer();
                draw_view(display, &view).expect("drawing into the OLED buffer must succeed");
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
            } else {
                render_pending = false;
                rendered_second = Some(visible_second);
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
        .expect("capabilities fit the protocol registry");
    capabilities
        .push(Capability::LiveStatus)
        .expect("capabilities fit the protocol registry");
    capabilities
        .push(Capability::ReadPresetCatalog)
        .expect("capabilities fit the protocol registry");
    capabilities
        .push(Capability::ProposePresetCatalog)
        .expect("capabilities fit the protocol registry");
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
    request_transfer_id: u16,
    now_ms: u64,
    session: &mut ProtocolSession,
    radio: &mut BleRadio,
    app: &App,
    catalog_updates: &mut CatalogUpdateCoordinator,
    catalog_revision: u64,
    pending_catalog_result: &mut Option<PresetCatalogResultEvent>,
    journal: Option<&PersistentJournal<NvsJournalStore, JOURNAL_CAPACITY>>,
    wall_clock: &mut VolatileClock,
    status_epoch: [u8; 8],
    status_revision: u64,
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
    log::info!(
        "BLE protocol request: transfer={} request_id={} kind={} version={}.{} bytes={}",
        request_transfer_id,
        request.request_id,
        request.request.message_kind(),
        request.version.major,
        request.version.minor,
        bytes.len()
    );
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
                match status_response_with_order(&status, Some((status_epoch, status_revision))) {
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
        ProtocolAction::ReadPresetCatalog {
            request_id,
            version,
        } => match protocol_catalog(app.catalog(), catalog_revision) {
            Ok(catalog) => ResponseEnvelope {
                version,
                request_id,
                response: Response::PresetCatalog(catalog),
            },
            Err(()) => protocol_error(request_id, version, ErrorCode::InternalError, Some(9), None),
        },
        ProtocolAction::ProposePresetCatalog {
            request_id,
            version,
            proposal,
        } => match proposed_catalog(&proposal.custom_entries) {
            Ok(catalog) => match catalog_updates.stage(
                now_ms,
                app.is_idle(),
                catalog_revision,
                proposal.expected_revision,
                proposal.proposal_id,
                catalog,
            ) {
                Ok(staged) => {
                    if let Some(replaced) = staged.replaced_proposal_id {
                        *pending_catalog_result = Some(PresetCatalogResultEvent {
                            proposal_id: replaced,
                            result: CatalogResult::Cancelled,
                            catalog_revision: None,
                        });
                    }
                    ResponseEnvelope {
                        version,
                        request_id,
                        response: Response::ProposePresetCatalog(ProposePresetCatalogResponse {
                            proposal_id: staged.proposal_id,
                            expires_in_ms: u32::try_from(
                                staged.expires_at_ms.saturating_sub(now_ms),
                            )
                            .unwrap_or(u32::MAX),
                        }),
                    }
                }
                Err(CatalogStageError::Busy) => {
                    protocol_error(request_id, version, ErrorCode::Busy, Some(11), None)
                }
                Err(CatalogStageError::Conflict { .. }) => protocol_error(
                    request_id,
                    version,
                    ErrorCode::CatalogConflict,
                    Some(11),
                    Some(0),
                ),
                Err(CatalogStageError::DeadlineOverflow) => protocol_error(
                    request_id,
                    version,
                    ErrorCode::InternalError,
                    Some(11),
                    None,
                ),
            },
            Err(()) => protocol_error(
                request_id,
                version,
                ErrorCode::InvalidField,
                Some(11),
                Some(2),
            ),
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
    let response_kind = match &response.response {
        Response::Hello(_) => 2,
        Response::Status(_) => 4,
        Response::SessionPage(_) => 6,
        Response::ClockAnchor(_) => 8,
        Response::PresetCatalog(_) => 10,
        Response::ProposePresetCatalog(_) => 12,
        Response::Error(_) => 255,
    };
    log::info!(
        "BLE protocol response: transfer={} request_id={} kind={} version={}.{} bytes={}",
        request_transfer_id,
        response.request_id,
        response_kind,
        response.version.major,
        response.version.minor,
        length
    );
    if let Err(error) = radio.queue_response(&encoded[..length], request_transfer_id) {
        log::warn!(
            "BLE logical response could not enter bounded outbox: request_id={} error={error:?}",
            response.request_id
        );
        radio.abandon_response();
    }
}

#[cfg(feature = "acceptance-diagnostic")]
fn log_resource_snapshot(phase: &str) {
    let snapshot = focus_esp_resources::snapshot();
    log::info!(
        "resource snapshot: phase={phase} heap_free_8bit={} heap_minimum_8bit={} main_stack_minimum_free={}",
        snapshot.heap_free_8bit,
        snapshot.heap_minimum_8bit,
        snapshot.current_stack_minimum_free
    );
}

#[cfg(not(feature = "acceptance-diagnostic"))]
fn log_resource_snapshot(_phase: &str) {}

fn mark_live_status(revision: &mut u64, pending: &mut bool) {
    *revision = revision.saturating_add(1);
    *pending = true;
}

fn ordered_status(
    app: &App,
    now_ms: u64,
    journal: Option<&PersistentJournal<NvsJournalStore, JOURNAL_CAPACITY>>,
    wall_clock: VolatileClock,
    status_epoch: [u8; 8],
    status_revision: u64,
) -> Option<focus_protocol::StatusResponse> {
    let journal = journal?;
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
        wall_clock,
    );
    status_response_with_order(&status, Some((status_epoch, status_revision))).ok()
}

fn queue_device_event(radio: &mut BleRadio, event: DeviceEvent) -> bool {
    let envelope = EventEnvelope {
        version: ProtocolVersion::CURRENT,
        event,
    };
    let mut encoded = [0; MAX_LOGICAL_MESSAGE_BYTES];
    let Ok(length) = encode_event(&envelope, &mut encoded) else {
        log::error!("BLE device event encoding failed");
        return false;
    };
    match radio.queue_latest_event(&encoded[..length]) {
        Ok(()) => true,
        Err(
            crate::ble_radio::QueueError::Disconnected
            | crate::ble_radio::QueueError::NotSubscribed,
        ) => false,
        Err(error) => {
            log::warn!("BLE device event could not enter latest-value outbox: {error:?}");
            false
        }
    }
}

fn protocol_catalog(catalog: &Catalog, revision: u64) -> Result<PresetCatalogResponse, ()> {
    let mut entries = heapless::Vec::new();
    for preset in catalog.presets() {
        entries
            .push(CatalogEntry {
                preset: PresetSnapshot {
                    id: preset.id.as_str().try_into().map_err(|()| ())?,
                    name: preset.name.as_str().try_into().map_err(|()| ())?,
                    planned_duration_ms: u32::try_from(preset.duration_ms).map_err(|_| ())?,
                },
                built_in: preset.built_in,
            })
            .map_err(|_| ())?;
    }
    Ok(PresetCatalogResponse { revision, entries })
}

fn proposed_catalog(custom_entries: &heapless::Vec<PresetSnapshot, 8>) -> Result<Catalog, ()> {
    let mut presets = runtime_built_ins();
    for entry in custom_entries {
        presets
            .push(
                Preset::custom(
                    entry.id.as_str(),
                    entry.name.as_str(),
                    u64::from(entry.planned_duration_ms),
                )
                .map_err(|_| ())?,
            )
            .map_err(|_| ())?;
    }
    Catalog::combined(presets.as_slice()).map_err(|_| ())
}

struct CatalogResolution {
    result: Option<PresetCatalogResultEvent>,
    effects: Option<Effects>,
    catalog_changed: bool,
}

fn resolve_catalog_action(
    action: CatalogConfirmationAction,
    app: &mut App,
    store: Option<&mut NvsPresetCatalogStore>,
    catalog_revision: &mut u64,
) -> CatalogResolution {
    let result_only = |proposal_id, result| CatalogResolution {
        result: Some(PresetCatalogResultEvent {
            proposal_id,
            result,
            catalog_revision: None,
        }),
        effects: None,
        catalog_changed: false,
    };
    match action {
        CatalogConfirmationAction::None => CatalogResolution {
            result: None,
            effects: None,
            catalog_changed: false,
        },
        CatalogConfirmationAction::Rejected { proposal_id } => {
            result_only(proposal_id, CatalogResult::Rejected)
        }
        CatalogConfirmationAction::Expired { proposal_id } => {
            result_only(proposal_id, CatalogResult::Expired)
        }
        CatalogConfirmationAction::Cancelled { proposal_id } => {
            result_only(proposal_id, CatalogResult::Cancelled)
        }
        CatalogConfirmationAction::Commit(commit) => {
            let Some(store) = store else {
                return result_only(commit.proposal_id, CatalogResult::StorageFailed);
            };
            if commit.expected_revision != *catalog_revision {
                return result_only(commit.proposal_id, CatalogResult::Cancelled);
            }
            let custom = &commit.catalog.presets()[5..];
            match commit_catalog(store, *catalog_revision, custom) {
                Ok(stored) => match app.replace_catalog(commit.catalog) {
                    Ok(effects) => {
                        *catalog_revision = stored.revision;
                        CatalogResolution {
                            result: Some(PresetCatalogResultEvent {
                                proposal_id: commit.proposal_id,
                                result: CatalogResult::Committed,
                                catalog_revision: Some(stored.revision),
                            }),
                            effects: Some(effects),
                            catalog_changed: true,
                        }
                    }
                    Err(error) => {
                        log::error!("durable catalog could not enter Idle app: {error:?}");
                        result_only(commit.proposal_id, CatalogResult::StorageFailed)
                    }
                },
                Err(error) => {
                    log::error!("preset catalog durable commit failed: {error:?}");
                    result_only(commit.proposal_id, CatalogResult::StorageFailed)
                }
            }
        }
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
) -> bool {
    let before_snapshot = app.snapshot(now_ms);
    let before = before_snapshot.state;
    let effects = app.handle(now_ms, event);
    let after = app.snapshot(now_ms).state;
    match outcome_queue.observe(before, after, now_ms, effects.outcome.clone(), wall_clock) {
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
    before_snapshot.state != app.snapshot(now_ms).state
        || before_snapshot.preset.id != app.snapshot(now_ms).preset.id
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
        match selection_persistence.selection_changed(now_ms, preset.clone()) {
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
