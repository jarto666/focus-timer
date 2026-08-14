use std::{thread, time::Duration};

use esp32_nimble::{BLEAdvertisementData, BLEDevice, NimbleProperties, uuid128};

const SERVICE_UUID: esp32_nimble::utilities::BleUuid =
    uuid128!("1cf47046-2e37-4642-a30e-df24879f994f");
const COMMAND_UUID: esp32_nimble::utilities::BleUuid =
    uuid128!("65ecdf0d-cde0-4543-a62b-c166c3341319");
const RESPONSE_UUID: esp32_nimble::utilities::BleUuid =
    uuid128!("2c4e304b-2581-481a-8646-89122d760711");

/// Minimal physical bring-up image for service discovery, command writes,
/// notifications, disconnect handling, and advertising restart.
pub(super) fn run() -> ! {
    let device = BLEDevice::take();
    let advertising = device.get_advertising();
    let server = device.get_server();
    server
        .on_connect(|_, connection| {
            log::info!(
                "BLE ECHO DIAGNOSTIC connected: handle={}",
                connection.conn_handle()
            );
        })
        .on_disconnect(|connection, reason| {
            log::info!(
                "BLE ECHO DIAGNOSTIC disconnected: handle={} reason={reason:?}; advertising restart requested",
                connection.conn_handle()
            );
        })
        .advertise_on_disconnect(true);

    let service = server.create_service(SERVICE_UUID);
    let response = service
        .lock()
        .create_characteristic(RESPONSE_UUID, NimbleProperties::NOTIFY);
    response.lock().on_subscribe(|_, connection, subscription| {
        log::info!(
            "BLE ECHO DIAGNOSTIC subscription changed: handle={} subscription={subscription:?}",
            connection.conn_handle()
        );
    });

    let command = service
        .lock()
        .create_characteristic(COMMAND_UUID, NimbleProperties::WRITE);
    command.lock().on_write(move |request| {
        let bytes = request.recv_data();
        let connection = request.desc().conn_handle();
        log::info!(
            "BLE ECHO DIAGNOSTIC command received: handle={connection} bytes={}",
            bytes.len()
        );
        match response.lock().notify_with(bytes, connection) {
            Ok(()) => log::info!(
                "BLE ECHO DIAGNOSTIC response notified: handle={connection} bytes={}",
                bytes.len()
            ),
            Err(error) => log::warn!(
                "BLE ECHO DIAGNOSTIC notification failed: handle={connection} error={error:?}"
            ),
        }
    });

    advertising
        .lock()
        .set_data(
            BLEAdvertisementData::new()
                .name("FocusTimer")
                .add_service_uuid(SERVICE_UUID),
        )
        .expect("Focus Timer advertising data must fit");
    advertising
        .lock()
        .start()
        .expect("Focus Timer advertising must start");
    log::warn!(
        "BLE ECHO DIAGNOSTIC advertising FocusTimer with the protocol service; subscribe to response, then write one value to command"
    );

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
