use std::sync::Arc;

use esp32_nimble::enums::SecurityIOCap;
use esp32_nimble::utilities::mutex::Mutex;
use esp32_nimble::utilities::BleUuid;
use esp32_nimble::BLEDevice;
use esp32_nimble::{uuid128, BLECharacteristic, BLEServer, NimbleProperties};

use crate::BluetoothMessage;

const BLE_APPEARANCE_GENERIC_HID: u16 = 960;
const SERVICE_UUID: BleUuid = uuid128!("fafafafa-fafa-fafa-fafa-fafafafafafa");
const LED_STATUS_CHARACTERISTIC_UUID: BleUuid = uuid128!("3c9a3f00-8ed3-4bdf-8a39-a01bebede295");
const NOTIFY_POSITION_CHARACTERISTIC_UUID: BleUuid =
    uuid128!("a3c87500-8ed3-4bdf-8a39-a01bebede295");

const DEVICE_NAME: &str = "H-Button";

pub struct Ble {
    server: &'static mut BLEServer,
    hid_characteristic: Arc<Mutex<BLECharacteristic>>,
    led_status_characteristic: Arc<Mutex<BLECharacteristic>>,
}

impl Ble {
    pub fn new() -> Self {
        let ble_device = BLEDevice::take();
        ble_device
            .security()
            .set_auth(true, true, true)
            .set_io_cap(SecurityIOCap::NoInputNoOutput);

        esp32_nimble::BLEDevice::set_device_name(DEVICE_NAME).unwrap();

        let server = ble_device.get_server();
        server
            .on_connect(|_| {
                ::log::info!("Client connected");
            })
            .on_disconnect(|_| {
                ::log::info!("Client disconnected");
                ble_device.get_advertising().start().unwrap();
            });
        let service = server.create_service(SERVICE_UUID);

        let hid_characteristic = service.lock().create_characteristic(
            NOTIFY_POSITION_CHARACTERISTIC_UUID,
            NimbleProperties::READ | NimbleProperties::NOTIFY | NimbleProperties::WRITE,
        );
        hid_characteristic.lock().on_read(move |_, _| {
            ::log::info!("Read from position characteristic.");
        });

        let led_status_characteristic = service.lock().create_characteristic(
            LED_STATUS_CHARACTERISTIC_UUID,
            NimbleProperties::READ | NimbleProperties::WRITE,
        );

        let ble_advertising = ble_device.get_advertising();
        ble_advertising
            .name(DEVICE_NAME)
            .add_service_uuid(SERVICE_UUID)
            .appearance(BLE_APPEARANCE_GENERIC_HID)
            .scan_response(true)
            .start()
            .unwrap();

        Self {
            server,
            hid_characteristic,
            led_status_characteristic,
        }
    }

    pub fn notify(&mut self) {
        ::log::info!("Notifying hid_characteristic!");
        self.hid_characteristic.lock().notify();
    }

    pub fn write(&mut self, msg: BluetoothMessage) {
        let msg = serde_json::to_string(&msg).unwrap();
        ::log::info!("Writing msg to hid_characteristic: {}", msg);
        self.hid_characteristic.lock().set_value(msg.as_bytes());
    }

    pub fn connected(&self) -> bool {
        self.server.connected_count() > 0
    }

    pub fn set_led_status_characteristic_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&[u8], &esp_idf_sys::ble_gap_conn_desc) + Send + Sync + 'static,
    {
        self.led_status_characteristic.lock().on_write(callback);
    }
}
