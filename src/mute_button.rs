use esp_idf_hal::{gpio::*, peripheral::Peripheral};
use esp_idf_sys::{
    xQueueGenericCreate, xQueueGiveFromISR, xQueueReceive, QueueHandle_t, TickType_t,
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::c_void,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::debouncer::Debouncer;
static mut EVENT_QUEUE: Option<QueueHandle_t> = None;

unsafe extern "C" fn notify_interrupt(_: *mut c_void) {
    xQueueGiveFromISR(EVENT_QUEUE.unwrap(), std::ptr::null_mut());
}

#[derive(Deserialize, Serialize, Debug)]
pub enum MuteButtonStatus {
    Pressed,
    Released,
}

pub struct MuteButton<I: InputPin> {
    _button: Arc<Mutex<PinDriver<'static, I, Input>>>,
}

impl<I> MuteButton<I>
where
    I: InputPin + OutputPin,
{
    pub fn new(
        button_pin: impl Peripheral<P = I> + 'static,
        debounce_duration: Duration,
        callback: impl Fn() + Send + 'static,
    ) -> Self {
        let mut button = PinDriver::input(button_pin).unwrap();
        button.set_pull(Pull::Up).unwrap();
        button.set_interrupt_type(InterruptType::NegEdge).unwrap();
        button.enable_interrupt().unwrap();

        let button = Arc::new(Mutex::new(button));

        const QUEUE_TYPE_BASE: u8 = 0;
        const ITEM_SIZE: u32 = 0;
        const QUEUE_SIZE: u32 = 10;

        unsafe {
            EVENT_QUEUE = Some(xQueueGenericCreate(QUEUE_SIZE, ITEM_SIZE, QUEUE_TYPE_BASE));
        }

        let debouncer = Debouncer::new(debounce_duration);

        let callback_button = {
            let debouncer = debouncer;

            move || {
                if debouncer.should_update() {
                    unsafe {
                        notify_interrupt(std::ptr::null_mut());
                    }
                }
            }
        };

        std::thread::spawn(move || loop {
            let mut message = MuteButtonStatus::Released;
            let message_ptr = &mut message as *mut _ as *mut c_void;
            let result = unsafe {
                xQueueReceive(
                    EVENT_QUEUE.unwrap(),
                    message_ptr,
                    (debounce_duration.as_millis() as u32).max(1) as TickType_t,
                )
            };
            if result != 0 {
                (callback)();
            }
        });

        unsafe {
            button.lock().unwrap().subscribe(callback_button).unwrap();
        }

        Self { _button: button }
    }
}
