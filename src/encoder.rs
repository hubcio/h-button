use esp_idf_hal::gpio::*;
use rotary_encoder_hal::{Direction, Rotary};
use std::{
    sync::{atomic, Arc, Mutex},
    time::Duration,
};

use crate::debouncer::Debouncer;

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
    ) -> Self {
        // todo refactor this to take gpio pins instead of pin drivers

        pin_driver_a
            .set_pull(Pull::Up)
            .expect("Failed to set pull-up for pin A");
        pin_driver_a
            .set_interrupt_type(InterruptType::AnyEdge)
            .expect("Failed to set interrupt type for pin A");
        pin_driver_a
            .enable_interrupt()
            .expect("Failed to enable interrupt for pin A");

        pin_driver_b
            .set_pull(Pull::Up)
            .expect("Failed to set pull-up for pin B");
        pin_driver_b
            .set_interrupt_type(InterruptType::AnyEdge)
            .expect("Failed to set interrupt type for pin B");
        pin_driver_b
            .enable_interrupt()
            .expect("Failed to enable interrupt for pin B");

        let encoder = Arc::new(Mutex::new(Rotary::new(pin_driver_a, pin_driver_b)));
        let position = Arc::new(atomic::AtomicI32::new(0));

        let debouncer_a = Debouncer::new(debounce_duration);
        let debouncer_b = Debouncer::new(debounce_duration);

        let callback_enc_gpio_a = {
            let enc = encoder.clone();
            let pos = position.clone();
            let debouncer = debouncer_a;
            move || {
                if debouncer.should_update() {
                    let mut enc = enc.lock().unwrap();
                    let direction = enc.update().unwrap();
                    drop(enc);

                    match direction {
                        Direction::Clockwise => {
                            pos.fetch_add(1, atomic::Ordering::SeqCst);
                        }
                        Direction::CounterClockwise => {
                            pos.fetch_sub(1, atomic::Ordering::SeqCst);
                        }
                        Direction::None => {}
                    }
                }
            }
        };

        let callback_enc_gpio_b = {
            let enc = encoder.clone();
            let pos = position.clone();
            let debouncer = debouncer_b;
            move || {
                if debouncer.should_update() {
                    let mut enc = enc.lock().unwrap();
                    let direction = enc.update().unwrap();
                    drop(enc);

                    match direction {
                        Direction::Clockwise => {
                            pos.fetch_add(1, atomic::Ordering::SeqCst);
                        }
                        Direction::CounterClockwise => {
                            pos.fetch_sub(1, atomic::Ordering::SeqCst);
                        }
                        Direction::None => {}
                    }
                }
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
