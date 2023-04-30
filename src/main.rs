// use std::ffi::c_void;
// use std::ptr;
use std::{ffi::c_void, time::Duration};

use esp_idf_hal::delay::FreeRtos;
#[allow(unused_imports)]
use esp_idf_sys::{
    self as _, esp, esp_random, gpio_config, gpio_config_t, gpio_install_isr_service,
    gpio_int_type_t_GPIO_INTR_POSEDGE, gpio_isr_handler_add, gpio_mode_t_GPIO_MODE_INPUT,
    xQueueGenericCreate, xQueueGiveFromISR, xQueueReceive, QueueHandle_t, ESP_INTR_FLAG_IRAM,
};

// This `static mut` holds the queue handle we are going to get from `xQueueGenericCreate`.
// This is unsafe, but we are careful not to enable our GPIO interrupt handler until after this value has been initialized, and then never modify it again
static mut EVENT_QUEUE: Option<QueueHandle_t> = None;

unsafe extern "C" fn notify_interrupt(_: *mut c_void) {
    xQueueGiveFromISR(EVENT_QUEUE.unwrap(), std::ptr::null_mut());
}

use esp_idf_hal::gpio::*;
// use esp_idf_svc::timer::EspTimerService;

use esp_idf_hal::peripherals::Peripherals;

mod ble_keyboard;
mod encoder;
mod led_button;

use ble_keyboard::BleKeyboard;
use encoder::Encoder;

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // WDT OFF
    unsafe {
        esp_idf_sys::esp_task_wdt_delete(esp_idf_sys::xTaskGetIdleTaskHandleForCPU(
            esp_idf_hal::cpu::core() as u32,
        ));
    };

    let mut ble_keyboard = BleKeyboard::new();

    println!("lets gooooo!");

    let peripherals = Peripherals::take().unwrap();

    let led_button = led_button::LedButton::new(
        peripherals.pins.gpio2,
        peripherals.pins.gpio13,
        Duration::from_millis(150),
    );

    let pin_a = PinDriver::input(peripherals.pins.gpio26).unwrap();
    let pin_b = PinDriver::input(peripherals.pins.gpio25).unwrap();

    let encoder = Encoder::new(
        pin_a,
        pin_b,
        Duration::from_millis(5),
        // on_change_callback_encoder,
    );

    const QUEUE_TYPE_BASE: u8 = 0;
    const ITEM_SIZE: u32 = 0; // we're not posting any actual data, just notifying
    const QUEUE_SIZE: u32 = 10;

    unsafe {
        // Instantiates the event queue
        EVENT_QUEUE = Some(xQueueGenericCreate(QUEUE_SIZE, ITEM_SIZE, QUEUE_TYPE_BASE));
    }

    let mut last_pos = encoder.position();
    let mut last_mute_state = led_button.is_led_on();

    loop {
        FreeRtos::delay_ms(10);

        if ble_keyboard.connected() {
            if last_mute_state != led_button.is_led_on() {
                ble_keyboard.toggle_mute();
                last_mute_state = led_button.is_led_on();
            }

            if last_pos != encoder.position() {
                if encoder.position() > last_pos {
                    ble_keyboard.volume_up();
                } else {
                    ble_keyboard.volume_down();
                }
                last_pos = encoder.position();
            }
        }
    }
}
