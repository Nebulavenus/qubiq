use std::time::{SystemTime, Duration};
use std::thread;

/// Keeps track of the time each tick takes and regulates the server ticks per second (TPS).
pub struct Clock {
    micros_ema: f32,
    full_tick_millis: u128,
    full_tick: Duration,
    time: SystemTime,
}

impl Clock {
    /// Creates a new clock with the given tick length in milliseconds.
    pub fn new(tick_length: u128) -> Self {
        Clock {
            micros_ema: 0_f32,
            full_tick_millis: tick_length,
            full_tick: Duration::from_millis(tick_length as u64),
            time: SystemTime::now(),
        }
    }

    /// Called at the start of a server tick
    pub fn start(&mut self) {
        self.time = SystemTime::now();
    }

    /// The tick code has finished executing, record the time and sleep if extra time remains
    pub fn finish_tick(&mut self) {
        match self.time.elapsed() {
            Ok(duration) => {
                self.micros_ema = (99_f32 * self.micros_ema + duration.as_micros() as f32) / 100_f32;

                if duration.as_millis() < self.full_tick_millis {
                    thread::sleep(self.full_tick - duration);
                }
            }
            Err(_) => thread::sleep(self.full_tick),
        }
    }

    /// Returns a buffered milliseconds per tick (MSPT) measurement.
    #[inline]
    pub fn mspt(&self) -> f32 {
        self.micros_ema / 1000_f32
    }

    /// Converts a milliseconds per tick value to ticks per second.
    #[inline]
    pub fn as_tps(&self, mspt: f32) -> f32 {
        if mspt < self.full_tick_millis as f32 {
            1000_f32 / (self.full_tick_millis as f32)
        } else {
            1000_f32 / mspt
        }
    }

    /// The maximum tps the server will tick at.
    #[inline]
    pub fn max_tps(&self) -> f32 {
        1000_f32 / self.full_tick_millis as f32
    }
}