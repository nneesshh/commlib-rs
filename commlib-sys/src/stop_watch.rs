//! Commlib: StopWatch
pub struct StopWatch {
    start: std::time::Instant,
}

impl StopWatch {
    ///
    pub fn new() -> StopWatch {
        StopWatch {
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
}
