#![cfg_attr(feature = "radio-failure-diagnostic", allow(dead_code))]

use std::{boxed::Box, sync::Arc, time::Instant};

use esp32_nimble::{
    BLEAdvertisementData, BLECharacteristic, BLEDevice, BLEError, NimbleProperties, NimbleSub,
    utilities::{BleUuid, mutex::Mutex},
    uuid128,
};
use focus_protocol::{
    FragmentError, MAX_LOGICAL_MESSAGE_BYTES, OwnedFragmenter, Reassembler, ReassemblyStatus,
    parse_frame,
};

const SERVICE_UUID: BleUuid = uuid128!("1cf47046-2e37-4642-a30e-df24879f994f");
const COMMAND_UUID: BleUuid = uuid128!("65ecdf0d-cde0-4543-a62b-c166c3341319");
const RESPONSE_UUID: BleUuid = uuid128!("2c4e304b-2581-481a-8646-89122d760711");
const PREFERRED_ATT_MTU: u16 = 185;
const MAX_GATT_VALUE_BYTES: usize = PREFERRED_ATT_MTU as usize - 3;

pub enum StartError {
    PreferredMtu(BLEError),
    AdvertisementData(BLEError),
    AdvertisementStart(BLEError),
}

impl core::fmt::Debug for StartError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PreferredMtu(error) => {
                formatter.debug_tuple("PreferredMtu").field(error).finish()
            }
            Self::AdvertisementData(error) => formatter
                .debug_tuple("AdvertisementData")
                .field(error)
                .finish(),
            Self::AdvertisementStart(error) => formatter
                .debug_tuple("AdvertisementStart")
                .field(error)
                .finish(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConnectionSnapshot {
    pub generation: u32,
    pub connected: bool,
    pub subscribed: bool,
    pub mtu: u16,
}

pub struct InboundMessage {
    bytes: [u8; MAX_LOGICAL_MESSAGE_BYTES],
    length: usize,
    pub connection_generation: u32,
    pub transfer_id: u16,
}

impl InboundMessage {
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.length]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueueError {
    Busy,
    Disconnected,
    NotSubscribed,
    Fragment(FragmentError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationProgress {
    Idle,
    Sent,
    Complete,
    DroppedConnection,
}

pub enum NotificationError {
    Fragment(FragmentError),
    Notify(BLEError),
}

impl core::fmt::Debug for NotificationError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Fragment(error) => formatter.debug_tuple("Fragment").field(error).finish(),
            Self::Notify(error) => formatter.debug_tuple("Notify").field(error).finish(),
        }
    }
}

#[derive(Clone, Copy)]
struct Connection {
    handle: u16,
    mtu: u16,
    subscribed: bool,
}

struct SharedState {
    generation: u32,
    connection: Option<Connection>,
    reassembler: Reassembler,
    pending: Option<InboundMessage>,
    outbound_busy: bool,
}

impl SharedState {
    const fn new() -> Self {
        Self {
            generation: 0,
            connection: None,
            reassembler: Reassembler::new(),
            pending: None,
            outbound_busy: false,
        }
    }

    fn next_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.generation = 1;
        }
    }

    fn reset_transfer(&mut self) {
        self.reassembler.reset();
        self.pending = None;
        self.outbound_busy = false;
    }
}

struct OutboundTransfer {
    connection_generation: u32,
    connection_handle: u16,
    fragments: OwnedFragmenter,
    logical_bytes: usize,
    frames_sent: usize,
    started_at: Instant,
}

/// Narrow production adapter around `NimBLE`. Callbacks only touch `SharedState`;
/// the single-owner firmware loop decodes requests and produces responses.
pub struct BleRadio {
    shared: Arc<Mutex<SharedState>>,
    response: Arc<Mutex<BLECharacteristic>>,
    outbound: Option<Box<OutboundTransfer>>,
    frame: [u8; MAX_GATT_VALUE_BYTES],
}

impl BleRadio {
    /// Starts the service and advertising. Advertising errors remain optional
    /// adapter failures so the timer runtime can continue without BLE.
    // Keeping service construction in one place makes it auditable that every
    // callback captures only the bounded shared mailbox.
    #[allow(clippy::too_many_lines)]
    pub fn start() -> Result<Self, StartError> {
        let device = BLEDevice::take();
        device
            .set_preferred_mtu(PREFERRED_ATT_MTU)
            .map_err(StartError::PreferredMtu)?;

        let shared = Arc::new(Mutex::new(SharedState::new()));
        let server = device.get_server();
        let connect_shared = shared.clone();
        server.on_connect(move |_, connection| {
            let mut state = connect_shared.lock();
            state.next_generation();
            state.connection = Some(Connection {
                handle: connection.conn_handle(),
                mtu: connection.mtu(),
                subscribed: false,
            });
            state.reset_transfer();
            log::info!(
                "BLE connected: generation={} handle={} mtu={}",
                state.generation,
                connection.conn_handle(),
                connection.mtu()
            );
        });
        let disconnect_shared = shared.clone();
        server
            .on_disconnect(move |connection, reason| {
                let mut state = disconnect_shared.lock();
                state.next_generation();
                state.connection = None;
                state.reset_transfer();
                log::info!(
                    "BLE disconnected: generation={} handle={} reason={reason:?}",
                    state.generation,
                    connection.conn_handle()
                );
            })
            .advertise_on_disconnect(true);

        let service = server.create_service(SERVICE_UUID);
        let response = service
            .lock()
            .create_characteristic(RESPONSE_UUID, NimbleProperties::NOTIFY);
        let subscribe_shared = shared.clone();
        response
            .lock()
            .on_subscribe(move |_, connection, subscription| {
                let mut state = subscribe_shared.lock();
                if let Some(active) = state.connection.as_mut()
                    && active.handle == connection.conn_handle()
                {
                    active.mtu = connection.mtu();
                    active.subscribed = subscription.contains(NimbleSub::NOTIFY);
                }
                log::info!(
                    "BLE response subscription: handle={} mtu={} enabled={}",
                    connection.conn_handle(),
                    connection.mtu(),
                    subscription.contains(NimbleSub::NOTIFY)
                );
            });

        let command = service
            .lock()
            .create_characteristic(COMMAND_UUID, NimbleProperties::WRITE);
        let write_shared = shared.clone();
        let ingress_epoch = Instant::now();
        command.lock().on_write(move |request| {
            let now_ms = u64::try_from(ingress_epoch.elapsed().as_millis()).unwrap_or(u64::MAX);
            let mut state = write_shared.lock();
            let connection_handle = request.desc().conn_handle();
            let connection_mtu = request.desc().mtu();
            let Some(connection) = state.connection.as_mut() else {
                request.reject();
                return;
            };
            if connection.handle != connection_handle || !connection.subscribed {
                request.reject();
                return;
            }
            connection.mtu = connection_mtu;
            if state.pending.is_some() || state.outbound_busy {
                log::warn!(
                    "BLE command rejected while one logical request is pending: handle={connection_handle}"
                );
                request.reject();
                return;
            }

            let frame_header = match parse_frame(request.recv_data()) {
                Ok((header, _)) => header,
                Err(error) => {
                    state.reassembler.reset();
                    log::warn!(
                        "BLE frame rejected and transfer reset: handle={connection_handle} error={error:?}"
                    );
                    request.reject();
                    return;
                }
            };
            match state.reassembler.accept_frame(request.recv_data(), now_ms) {
                Ok(ReassemblyStatus::InProgress) => {}
                Ok(ReassemblyStatus::Complete { message_length }) => {
                    let mut bytes = [0; MAX_LOGICAL_MESSAGE_BYTES];
                    let message = state
                        .reassembler
                        .completed_message()
                        .expect("complete status must expose one bounded message");
                    bytes[..message_length].copy_from_slice(message);
                    state.pending = Some(InboundMessage {
                        bytes,
                        length: message_length,
                        connection_generation: state.generation,
                        transfer_id: frame_header.transfer_id,
                    });
                    state.reassembler.reset();
                    log::debug!(
                        "BLE logical request queued: generation={} bytes={message_length}",
                        state.generation
                    );
                }
                Err(error) => {
                    log::warn!(
                        "BLE frame rejected and transfer reset: handle={connection_handle} error={error:?}"
                    );
                    request.reject();
                }
            }
        });

        let advertising = device.get_advertising();
        advertising
            .lock()
            .set_data(
                BLEAdvertisementData::new()
                    .name("FocusTimer")
                    .add_service_uuid(SERVICE_UUID),
            )
            .map_err(StartError::AdvertisementData)?;
        advertising
            .lock()
            .start()
            .map_err(StartError::AdvertisementStart)?;
        log::info!(
            "BLE advertising started: name=FocusTimer preferred_mtu={PREFERRED_ATT_MTU} max_connections=1"
        );

        Ok(Self {
            shared,
            response,
            outbound: None,
            frame: [0; MAX_GATT_VALUE_BYTES],
        })
    }

    #[must_use]
    pub fn connection_snapshot(&self) -> ConnectionSnapshot {
        let state = self.shared.lock();
        let connection = state.connection;
        ConnectionSnapshot {
            generation: state.generation,
            connected: connection.is_some(),
            subscribed: connection.is_some_and(|active| active.subscribed),
            mtu: connection.map_or(23, |active| active.mtu),
        }
    }

    /// Expires partial ingress and transfers at most one completed request to
    /// the single-owner runtime.
    pub fn take_request(&mut self, now_ms: u64) -> Option<InboundMessage> {
        let mut state = self.shared.lock();
        if state.reassembler.expire(now_ms) {
            log::warn!("BLE incomplete request expired and was reset");
        }
        let pending = state.pending.take();
        if pending.is_some() {
            state.outbound_busy = true;
        }
        pending
    }

    /// Releases a claimed request when decoding or response construction
    /// fails before an outbound transfer can be queued.
    pub fn abandon_response(&mut self) {
        if self.outbound.is_none() {
            self.shared.lock().outbound_busy = false;
        }
    }

    /// Starts one bounded response transfer for the active subscribed link.
    pub fn queue_response(
        &mut self,
        message: &[u8],
        request_transfer_id: u16,
    ) -> Result<(), QueueError> {
        if self.outbound.is_some() {
            return Err(QueueError::Busy);
        }
        let mut state = self.shared.lock();
        let connection = state.connection.ok_or(QueueError::Disconnected)?;
        if !connection.subscribed {
            return Err(QueueError::NotSubscribed);
        }
        let frame_bytes = usize::from(connection.mtu.saturating_sub(3)).min(MAX_GATT_VALUE_BYTES);
        let fragments = OwnedFragmenter::new(message, request_transfer_id, frame_bytes)
            .map_err(QueueError::Fragment)?;
        self.outbound = Some(Box::new(OutboundTransfer {
            connection_generation: state.generation,
            connection_handle: connection.handle,
            fragments,
            logical_bytes: message.len(),
            frames_sent: 0,
            started_at: Instant::now(),
        }));
        state.outbound_busy = true;
        Ok(())
    }

    /// Emits at most one notification. A failed or stale transfer is discarded;
    /// the client may safely retry the read-only logical request.
    pub fn poll_notification(&mut self) -> Result<NotificationProgress, NotificationError> {
        if self.outbound.is_none() {
            return Ok(NotificationProgress::Idle);
        }
        let snapshot = self.connection_snapshot();
        let outbound = self
            .outbound
            .as_mut()
            .expect("outbound presence was checked before connection snapshot");
        if !snapshot.connected
            || !snapshot.subscribed
            || snapshot.generation != outbound.connection_generation
        {
            self.outbound = None;
            self.shared.lock().outbound_busy = false;
            return Ok(NotificationProgress::DroppedConnection);
        }

        let next_frame = match outbound.fragments.next_frame(&mut self.frame) {
            Ok(next_frame) => next_frame,
            Err(error) => {
                self.outbound = None;
                self.shared.lock().outbound_busy = false;
                return Err(NotificationError::Fragment(error));
            }
        };
        let Some(length) = next_frame else {
            self.outbound = None;
            self.shared.lock().outbound_busy = false;
            return Ok(NotificationProgress::Complete);
        };
        if let Err(error) = self
            .response
            .lock()
            .notify_with(&self.frame[..length], outbound.connection_handle)
        {
            self.outbound = None;
            self.shared.lock().outbound_busy = false;
            return Err(NotificationError::Notify(error));
        }
        outbound.frames_sent += 1;
        if outbound.fragments.is_complete() {
            log::info!(
                "BLE logical response transfer: bytes={} frames={} elapsed_ms={}",
                outbound.logical_bytes,
                outbound.frames_sent,
                outbound.started_at.elapsed().as_millis()
            );
            self.outbound = None;
            self.shared.lock().outbound_busy = false;
            Ok(NotificationProgress::Complete)
        } else {
            Ok(NotificationProgress::Sent)
        }
    }
}
