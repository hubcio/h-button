use std::ffi::c_void;
use std::ptr;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use esp_idf_hal::delay::FreeRtos;
#[allow(unused_imports)]
use esp_idf_sys::{
    self as _, esp, esp_random, gpio_config, gpio_config_t, gpio_install_isr_service,
    gpio_int_type_t_GPIO_INTR_POSEDGE, gpio_isr_handler_add, gpio_mode_t_GPIO_MODE_INPUT,
    xQueueGenericCreate, xQueueGiveFromISR, xQueueReceive, QueueHandle_t, ESP_INTR_FLAG_IRAM,
};

// This `static mut` holds the queue handle we are going to get from `xQueueGenericCreate`.
// This is unsafe, but we are careful not to enable our GPIO interrupt handler until after this value has been initialised, and then never modify it again
static mut EVENT_QUEUE: Option<QueueHandle_t> = None;

unsafe extern "C" fn notify_interrupt(_: *mut c_void) {
    xQueueGiveFromISR(EVENT_QUEUE.unwrap(), std::ptr::null_mut());
}

use esp32_nimble::{uuid128, BLEDevice, NimbleProperties};

// use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_svc::timer::EspTimerService;

use esp_idf_hal::peripherals::Peripherals;

mod encoder;
mod led_button;

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

    println!("lets gooooo!");

    let ble_device = BLEDevice::take();

    let server = ble_device.get_server();

    server.on_connect(|_| {
        ::log::info!("Client connected");
        ::log::info!("Multi-connect support: start advertising");
        ble_device.get_advertising().start().unwrap();
    });
    let service = server.create_service(uuid128!("fafafafa-fafa-fafa-fafa-fafafafafafa"));

    // A static characteristic.
    let static_characteristic = service.lock().create_characteristic(
        uuid128!("d4e0e0d0-1a2b-11e9-ab14-d663bd873d93"),
        NimbleProperties::READ,
    );
    static_characteristic
        .lock()
        .set_value("Hello, world!".as_bytes());

    // A writable characteristic.
    let writable_characteristic = service.lock().create_characteristic(
        uuid128!("3c9a3f00-8ed3-4bdf-8a39-a01bebede295"),
        NimbleProperties::READ | NimbleProperties::WRITE,
    );
    writable_characteristic
        .lock()
        .on_read(move |_, _| {
            ::log::info!("Read from writable characteristic.");
        })
        .on_write(move |value, _param| {
            ::log::info!("Wrote to writable characteristic: {:?}", value);
        });
    let ble_advertising = ble_device.get_advertising();
    ble_advertising
        .name("H-Button")
        .add_service_uuid(uuid128!("fafafafa-fafa-fafa-fafa-fafafafafafa"));

    ble_advertising.start().unwrap();

    let peripherals = Peripherals::take().unwrap();
    // let led = Arc::new(Mutex::new(PinDriver::output(peripherals.pins.gpio2)?));
    // let switch = Arc::new(Mutex::new(PinDriver::input(peripherals.pins.gpio13)?));
    // switch.lock().unwrap().set_pull(Pull::Up)?;
    // switch
    //     .lock()
    //     .unwrap()
    //     .set_interrupt_type(InterruptType::NegEdge)?;
    // switch.lock().unwrap().enable_interrupt()?;
    // let timer = EspTimerService::new()
    //     .unwrap()
    //     .timer({
    //         let switch = switch.clone();
    //         move || {
    //             ::log::info!("Mute button click detected");
    //             switch.lock().unwrap().enable_interrupt().unwrap();
    //         }
    //     })
    //     .unwrap();

    // let callback_switch = {
    //     let switch = switch.clone();
    //     move || {
    // switch.lock().unwrap().disable_interrupt().unwrap();
    // led.lock().unwrap().toggle().unwrap();
    // timer.after(Duration::from_millis(150)).unwrap();
    //     }
    // };

    // unsafe {
    //     switch.lock().unwrap().subscribe(|| {
    //         switch.lock().unwrap().disable_interrupt().unwrap();
    //         led.lock().unwrap().toggle().unwrap();
    //         timer.after(Duration::from_millis(150)).unwrap();
    //         notify_interrupt(std::ptr::null_mut());
    //     })?;
    // }

    let led_button = led_button::LedButton::new(
        peripherals.pins.gpio2,
        peripherals.pins.gpio13,
        Duration::from_millis(150),
    );

    let pin_a = PinDriver::input(peripherals.pins.gpio25).unwrap();
    let pin_b = PinDriver::input(peripherals.pins.gpio26).unwrap();

    // A characteristic that notifies every second.
    let notifying_characteristic = service.lock().create_characteristic(
        uuid128!("a3c87500-8ed3-4bdf-8a39-a01bebede295"),
        NimbleProperties::READ | NimbleProperties::NOTIFY,
    );

    let on_change_callback_encoder = Arc::new({ move |pos: i32| {} });

    let encoder = Encoder::new(
        pin_a,
        pin_b,
        Duration::from_millis(10),
        on_change_callback_encoder,
    );

    // Queue configurations
    const QUEUE_TYPE_BASE: u8 = 0;
    const ITEM_SIZE: u32 = 0; // we're not posting any actual data, just notifying
    const QUEUE_SIZE: u32 = 10;

    unsafe {
        // Instantiates the event queue
        EVENT_QUEUE = Some(xQueueGenericCreate(QUEUE_SIZE, ITEM_SIZE, QUEUE_TYPE_BASE));
    }

    let mut last_pos = encoder.position();
    let mut last_button_state = led_button.is_led_on();

    loop {
        FreeRtos::delay_ms(500);

        let characteristic_payload = format!(
            "Position: {}, Switch: {}",
            encoder.position(),
            led_button.is_led_on()
        );
        ::log::info!("{}", characteristic_payload);

        if (last_pos != encoder.position()) || (last_button_state != led_button.is_led_on()) {
            ::log::info!("sending notification");
            notifying_characteristic
                .lock()
                .set_value(characteristic_payload.as_bytes())
                .notify();
        }

        last_pos = encoder.position();
        last_button_state = led_button.is_led_on();

        // freertos sleep

        // unsafe {
        //     const QUEUE_WAIT_TICKS: u32 = 1000;

        //     // Reads the event item out of the queue
        //     let res = xQueueReceive(EVENT_QUEUE.unwrap(), ptr::null_mut(), QUEUE_WAIT_TICKS);

        //     if res > 0 {
        //         ::log::info!(
        //             "received notification pos: {:?}, res: {}",
        //             encoder.position(),
        //             res
        //         );
        //         notifying_characteristic
        //             .lock()
        //             .set_value(
        //                 format!(
        //                     "XD" // "pos: {:?}, button: {:?}",
        //                          // encoder.position(),
        //                          // led.lock().unwrap().get_level()
        //                 )
        //                 .as_bytes(),
        //             )
        //             .notify();
        //     }
        // }

        // let mut pos = lock.lock().unwrap();

        // while *pos == last_pos {
        //     pos = cvar.wait(pos).unwrap();
        // }
        // ::log::info!("pos: {:?}", encoder.position());

        // notifying_characteristic
        //     .lock()
        //     .set_value(format!("pos: {:?}", encoder.position()).as_bytes())
        //     .notify();

        // last_pos = *pos;
    }
}
