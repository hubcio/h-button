use std::{sync::Arc, time::Duration};

use esp_idf_hal::delay::FreeRtos;

#[allow(unused_imports)]
use esp_idf_sys::{
    self as _, esp, esp_random, gpio_config, gpio_config_t, gpio_install_isr_service,
    gpio_int_type_t_GPIO_INTR_POSEDGE, gpio_isr_handler_add, gpio_mode_t_GPIO_MODE_INPUT,
    xQueueGenericCreate, xQueueGiveFromISR, xQueueReceive, QueueHandle_t, ESP_INTR_FLAG_IRAM,
};

use esp_idf_hal::gpio::*;

use esp_idf_hal::peripherals::Peripherals;

mod ble;
mod debouncer;
mod encoder;
mod mute_button;
mod mute_led;

use encoder::Encoder;
use mute_led::LedStatus;
use serde::{Deserialize, Serialize};

use crate::{ble::*, mute_button::*, mute_led::MuteLed};

#[derive(Deserialize, Serialize, Debug)]
pub enum BluetoothMessage {
    HidStatus(HidStatus), // from server (esp32) to client (windows, mac os, linux)
    SetMicMuteIndicator(LedStatus), // from client to server
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HidStatus {
    pub encoder_position: i32,
    pub mic_mute_button_press_count: u32,
    pub led_status: LedStatus,
}
fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    ::log::info!("Let's gooooo!");

    unsafe {
        esp_idf_sys::esp_task_wdt_delete(esp_idf_sys::xTaskGetIdleTaskHandleForCPU(
            esp_idf_hal::cpu::core() as u32,
        ));
    };

    let mut ble = Ble::new();

    let peripherals = Peripherals::take().unwrap();
    let pin_a = PinDriver::input(peripherals.pins.gpio9).unwrap();
    let pin_b = PinDriver::input(peripherals.pins.gpio10).unwrap();
    let encoder = Encoder::new(pin_a, pin_b, Duration::from_millis(2));
    let mute_led = Arc::new(MuteLed::new(21));

    let no_callback = None::<Arc<dyn Fn() + Send + Sync + 'static>>;
    let mute_button = MuteButton::new(
        peripherals.pins.gpio8,
        Duration::from_millis(150),
        no_callback,
    );
    let mute_led_clone = mute_led.clone();
    ble.set_led_status_characteristic_callback(move |value, _| {
        let led_status: Result<BluetoothMessage, _> = serde_json::from_slice(value);
        ::log::info!("Received msg mute LED: {:?}", led_status);
        match led_status {
            Ok(BluetoothMessage::SetMicMuteIndicator(status)) => match status {
                LedStatus::On => mute_led_clone.set_led_on(),
                LedStatus::Off => mute_led_clone.set_led_off(),
            },
            Err(e) => {
                ::log::info!("Failed to deserialize LED status message: {}", e);
            }
            _ => {}
        }
    });

    let mut last_position = encoder.position();
    let mut last_press_count = mute_button.press_count();

    let init_msg = BluetoothMessage::HidStatus(HidStatus {
        encoder_position: last_position,
        mic_mute_button_press_count: last_press_count,
        led_status: Default::default(),
    });
    ble.write(init_msg);

    ::log::info!("Entering eternal loop");

    loop {
        FreeRtos::delay_ms(100);

        if ble.connected()
            && (last_position != encoder.position()
                || last_press_count != mute_button.press_count())
        {
            let msg = BluetoothMessage::HidStatus(HidStatus {
                encoder_position: encoder.position(),
                mic_mute_button_press_count: mute_button.press_count(),
                led_status: Default::default(),
            });
            ble.write(msg);
            ble.notify();
        }

        last_position = encoder.position();
        last_press_count = mute_button.press_count();
    }
}
