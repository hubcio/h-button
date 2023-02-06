use esp_idf_hal::{gpio::*, peripheral::Peripheral};
use esp_idf_svc::timer::EspTimerService;
// use esp_idf_svc::timer::EspTimerService;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

pub struct LedButton<O: OutputPin, I: InputPin> {
    led: Arc<Mutex<PinDriver<'static, O, Output>>>,
    button: Arc<Mutex<PinDriver<'static, I, Input>>>,
    led_state: Arc<AtomicBool>,
}

impl<O, I> LedButton<O, I>
where
    O: InputPin + OutputPin,
    I: InputPin + OutputPin,
{
    pub fn new(
        led_pin: impl Peripheral<P = O> + 'static,
        button_pin: impl Peripheral<P = I> + 'static,
        debouce_duration: Duration,
    ) -> Self {
        let mut led = PinDriver::output(led_pin).unwrap();
        led.set_level(Level::Low).unwrap();

        let mut button = PinDriver::input(button_pin).unwrap();
        button.set_pull(Pull::Up).unwrap();
        button.set_interrupt_type(InterruptType::NegEdge).unwrap();
        button.enable_interrupt().unwrap();

        let led = Arc::new(Mutex::new(led));
        let button = Arc::new(Mutex::new(button));
        let led_state = Arc::new(AtomicBool::new(false));

        let callback_timer = {
            let button = button.clone();
            move || {
                ::log::info!("Mute button click detected");
                button.lock().unwrap().enable_interrupt().unwrap();
            }
        };

        let timer = Arc::new(
            EspTimerService::new()
                .unwrap()
                .timer(callback_timer)
                .unwrap(),
        );

        let callback_button = {
            let switch = button.clone();
            let led = led.clone();
            let led_state = led_state.clone();
            move || {
                switch.lock().unwrap().disable_interrupt().unwrap();
                // led.lock().unwrap().toggle().unwrap();
                let current_led_state = led_state.load(Ordering::Relaxed);
                led_state.store(!current_led_state, Ordering::Relaxed);
                led.lock()
                    .unwrap()
                    .set_level(if current_led_state {
                        Level::Low // todo fix if broken
                    } else {
                        Level::High
                    })
                    .unwrap();
                timer.after(debouce_duration).unwrap();
            }
        };

        unsafe {
            button.lock().unwrap().subscribe(callback_button).unwrap();
        }

        Self {
            led,
            button,
            led_state,
        }
    }

    pub fn is_led_on(&self) -> bool {
        self.led_state.load(Ordering::Relaxed)
    }
}
