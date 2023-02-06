use esp_idf_hal::gpio::*;
use esp_idf_svc::timer::EspTimerService;
use rotary_encoder_hal::{Direction, Rotary};
use std::{
    sync::{atomic, Arc, Mutex},
    time::Duration,
};

type RotaryEncoder<A, B, Input> =
    Arc<Mutex<Rotary<PinDriver<'static, A, Input>, PinDriver<'static, B, Input>>>>;

pub struct Encoder<A: Pin, B: Pin> {
    _encoder: RotaryEncoder<A, B, Input>,
    position: Arc<atomic::AtomicI32>,
}

impl<A, B> Encoder<A, B>
where
    A: InputPin + OutputPin,
    B: InputPin + OutputPin,
{
    pub fn new(
        mut pin_driver_a: PinDriver<'static, A, Input>,
        mut pin_driver_b: PinDriver<'static, B, Input>,
        debounce_duration: Duration,
        on_change_callback: Arc<impl Fn(i32) + 'static>,
    ) -> Self {
        pin_driver_a.set_pull(Pull::Up).unwrap();
        pin_driver_a
            .set_interrupt_type(InterruptType::AnyEdge)
            .unwrap();
        pin_driver_a.enable_interrupt().unwrap();

        pin_driver_b.set_pull(Pull::Up).unwrap();
        pin_driver_b
            .set_interrupt_type(InterruptType::AnyEdge)
            .unwrap();
        pin_driver_b.enable_interrupt().unwrap();

        let encoder = Arc::new(Mutex::new(Rotary::new(pin_driver_a, pin_driver_b)));
        let position = Arc::new(atomic::AtomicI32::new(0));

        let callback_timer_a = {
            let enc = encoder.clone();
            move || {
                enc.lock().unwrap().pin_a().enable_interrupt().unwrap();
            }
        };

        let timer_a = EspTimerService::new()
            .unwrap()
            .timer(callback_timer_a)
            .unwrap();

        let callback_timer_b = {
            let enc = encoder.clone();
            move || {
                enc.lock().unwrap().pin_b().enable_interrupt().unwrap();
            }
        };

        let timer_b = EspTimerService::new()
            .unwrap()
            .timer(callback_timer_b)
            .unwrap();
        // let on_change_callback = on_change_callback;

        let on_change_callback_a = on_change_callback.clone();
        let callback_enc_gpio_a = {
            let enc = encoder.clone();
            let pos = position.clone();
            move || {
                enc.lock().unwrap().pin_a().disable_interrupt().unwrap();

                let mut enc = enc.lock().unwrap();
                let direction = enc.update().unwrap();

                match direction {
                    Direction::Clockwise => {
                        pos.fetch_add(1, atomic::Ordering::SeqCst);
                    }
                    Direction::CounterClockwise => {
                        pos.fetch_sub(1, atomic::Ordering::SeqCst);
                    }
                    Direction::None => {}
                }
                on_change_callback_a(pos.load(atomic::Ordering::SeqCst));
                timer_a.after(debounce_duration).unwrap();
            }
        };

        let on_change_callback_b = on_change_callback;
        let callback_enc_gpio_b = {
            let enc = encoder.clone();
            let pos = position.clone();
            move || {
                enc.lock().unwrap().pin_b().disable_interrupt().unwrap();

                let mut enc = enc.lock().unwrap();
                let direction = enc.update().unwrap();

                match direction {
                    Direction::Clockwise => {
                        pos.fetch_add(1, atomic::Ordering::SeqCst);
                    }
                    Direction::CounterClockwise => {
                        pos.fetch_sub(1, atomic::Ordering::SeqCst);
                    }
                    Direction::None => {}
                }
                on_change_callback_b(pos.load(atomic::Ordering::SeqCst));
                timer_b.after(debounce_duration).unwrap();
            }
        };

        unsafe {
            encoder
                .lock()
                .unwrap()
                .pin_a()
                .subscribe(callback_enc_gpio_a)
                .unwrap();

            encoder
                .lock()
                .unwrap()
                .pin_b()
                .subscribe(callback_enc_gpio_b)
                .unwrap();
        }

        Self {
            _encoder: encoder,
            position,
        }
    }

    pub fn position(&self) -> i32 {
        self.position.load(atomic::Ordering::SeqCst)
    }
}
