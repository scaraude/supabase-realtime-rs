use std::time::Duration;
use tokio::time::sleep;

/// Timer for reconnection logic with exponential backoff
pub struct Timer {
    attempts: u32,
    intervals: Vec<u64>,
}

impl Timer {
    pub fn new(intervals: Vec<u64>) -> Self {
        Self {
            attempts: 0,
            intervals,
        }
    }

    /// Get the next delay duration
    pub fn next_delay(&mut self) -> Duration {
        let delay = if (self.attempts as usize) < self.intervals.len() {
            self.intervals[self.attempts as usize]
        } else {
            *self.intervals.last().unwrap_or(&10000)
        };

        self.attempts += 1;
        Duration::from_millis(delay)
    }

    /// Reset the timer
    pub fn reset(&mut self) {
        self.attempts = 0;
    }

    /// Schedule timeout with exponential backoff
    pub async fn schedule_timeout(&mut self) {
        let delay = self.next_delay();
        sleep(delay).await;
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new(vec![1000, 2000, 5000, 10000])
    }
}
