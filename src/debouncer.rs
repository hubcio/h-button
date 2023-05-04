// New Debouncer module
use esp_idf_sys::esp_timer_get_time;
use std::{
    sync::{atomic, Arc},
    time::Duration,
};

pub struct Debouncer {
    last_update: Arc<atomic::AtomicI64>,
    debounce_duration: Duration,
}

impl Debouncer {
    pub fn new(debounce_duration: Duration) -> Self {
        Self {
            last_update: Arc::new(atomic::AtomicI64::new(0)),
            debounce_duration,
        }
    }

    pub fn should_update(&self) -> bool {
        let now = unsafe { esp_timer_get_time() };
        let last_update = self.last_update.load(atomic::Ordering::SeqCst);
        if now - last_update >= self.debounce_duration.as_micros() as i64 {
            self.last_update.store(now, atomic::Ordering::SeqCst);
            true
        } else {
            false
        }
    }
}
