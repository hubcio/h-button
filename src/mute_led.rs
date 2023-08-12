use serde::{Deserialize, Serialize};
use smart_leds::{SmartLedsWrite, White};
use std::sync::{Arc, Mutex, RwLock};
use ws2812_esp32_rmt_driver::{driver::color::LedPixelColorGrbw32, LedPixelEsp32Rmt, RGBW8};

#[derive(Default, Deserialize, Serialize, Debug, Clone, Copy)]
pub enum LedStatus {
    On,
    #[default]
    Off,
}

pub struct MuteLed {
    ws2812: Arc<Mutex<LedPixelEsp32Rmt<RGBW8, LedPixelColorGrbw32>>>,
    led_state: Arc<RwLock<LedStatus>>,
}

impl MuteLed {
    pub fn new(led_pin: u32) -> Self {
        let ws2812: LedPixelEsp32Rmt<
            smart_leds::RGBA<u8, White<u8>>,
            ws2812_esp32_rmt_driver::driver::color::LedPixelColorImpl<4, 1, 0, 2, 3>,
        > = LedPixelEsp32Rmt::<RGBW8, LedPixelColorGrbw32>::new(0, led_pin).unwrap();

        let ws2812 = Arc::new(Mutex::new(ws2812));
        let led_state = Arc::new(RwLock::new(LedStatus::Off));

        Self { ws2812, led_state }
    }

    pub fn set_led_on(&self) {
        let pixels = std::iter::once(RGBW8::from((255, 0, 0, White(0))));
        self.ws2812.lock().unwrap().write(pixels).unwrap();

        let mut led_state = self.led_state.write().unwrap();
        *led_state = LedStatus::On;
    }

    pub fn set_led_off(&self) {
        let pixels = std::iter::once(RGBW8::from((0, 0, 0, White(0))));
        self.ws2812.lock().unwrap().write(pixels).unwrap();

        let mut led_state = self.led_state.write().unwrap();
        *led_state = LedStatus::Off;
    }

    pub fn _get_led_status(&self) -> LedStatus {
        *self.led_state.read().unwrap()
    }
}
