use esp_idf_hal::{gpio::*, peripheral::Peripheral};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, RwLock};

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum LedStatus {
    On,
    Off,
}

pub struct MuteLed<O: OutputPin> {
    _led: Arc<Mutex<PinDriver<'static, O, Output>>>,
    led_state: Arc<RwLock<LedStatus>>,
}

impl<O> MuteLed<O>
where
    O: InputPin + OutputPin,
{
    pub fn new(led_pin: impl Peripheral<P = O> + 'static) -> Self {
        let mut led = PinDriver::output(led_pin).unwrap();
        led.set_level(Level::Low).unwrap();

        let led = Arc::new(Mutex::new(led));
        let led_state = Arc::new(RwLock::new(LedStatus::Off));

        Self {
            _led: led,
            led_state,
        }
    }

    pub fn set_led_on(&self) {
        let mut led_state = self.led_state.write().unwrap();
        *led_state = LedStatus::On;
        self._led.lock().unwrap().set_level(Level::High).unwrap();
    }

    pub fn set_led_off(&self) {
        let mut led_state = self.led_state.write().unwrap();
        *led_state = LedStatus::Off;
        self._led.lock().unwrap().set_level(Level::Low).unwrap();
    }

    pub fn get_led_status(&self) -> LedStatus {
        *self.led_state.read().unwrap()
    }

    // pub fn is_led_on(&self) -> LedStatus {
    //     *self.led_state.read().unwrap()
    // }
}
