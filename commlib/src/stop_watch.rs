//! Commlib: StopWatch

use std::ops::Add;
pub struct StopWatch {
    start: std::time::Instant,
}

impl StopWatch {
    ///
    pub fn new() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }

    ///
    pub fn reset(&mut self) {
        self.start = std::time::Instant::now();
    }

    ///
    pub fn elapsed(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    ///
    pub fn elapsed_and_reset(&mut self) -> u128 {
        let now = std::time::Instant::now();
        let d = now.duration_since(self.start);
        self.start = self.start.add(d);
        d.as_millis()
    }
}
