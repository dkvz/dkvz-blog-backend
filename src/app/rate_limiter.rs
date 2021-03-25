use crate::utils::time_utils::current_timestamp;
/*
// Max value fot the counter before blocking:
pub const MAX_REQUESTS: u32 = 150;
// We rate limit if max requests is reached in that timeframe (seconds):
pub const MAX_REQUESTS_TIME: u32 = 60;
// Block duration in seconds:
pub const BLOCK_TIME: u32 = 60;
*/

/**
 * Just count the amount of times sensible endpoints are
 * being called per unit of time, supposed to block them
 * entirely for a specific "block time" when that happens.
 */
pub struct BasicRateLimiter {
  counter: u32,
  last_update: i64,
  is_limited: bool,
  max_requests: u32,
  // In seconds
  max_requests_time: u32,
  // In seconds
  block_duration: u32
}

impl BasicRateLimiter {

  pub fn new(
    max_requests: u32, 
    max_requests_time: u32, 
    block_duration: u32
  ) -> Self {
    Self {
      counter: 0,
      last_update: current_timestamp(),
      is_limited: false,
      max_requests,
      max_requests_time,
      block_duration
    }
  }
 
  pub fn is_locked(&self) -> bool {
    self.is_limited
  }

  pub fn is_expired(&self) -> bool {
    // If currently locked, check if past block_duration.
    // Check if past max_request_time otherwise.
    if self.is_locked() {
      current_timestamp() - self.last_update >= self.block_duration.into()
    } else {
      current_timestamp() - self.last_update >= self.max_requests_time.into()
    }
  }

  // I'm trying to finely separate what is mutable and what isn't.
  // Returns true if we're locked, false otherwise.
  pub fn update(&mut self) -> bool {
    // If we're locked, check if lock has expired:
    if self.is_expired() {
      // Reset:
      self.counter = 0;
      self.last_update = current_timestamp();
      self.is_limited = false;
    } else {
      self.counter += 1;
      // Are we above the rate limit?
      if self.counter >= self.max_requests {
        self.is_limited = true;
        // Reset last_update:
        self.last_update = current_timestamp();
      }
    }
    self.is_limited
  }

}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn basic_rate_limiter_initial_state() {
    let sut = BasicRateLimiter::new(2, 60, 60);
    assert_eq!(sut.is_expired(), false);
    assert_eq!(sut.is_locked(), false);
  }

  #[test]
  fn basic_rate_limiter_can_unlock() {
    // Block for 0 seconds:
    let mut sut = BasicRateLimiter::new(2, u32::MAX, 0);
    for _ in 0..2 {
      sut.update();
    }
    assert_eq!(sut.is_locked(), true);
    sut.update();
    // Should be unlocked now:
    assert_eq!(sut.is_locked(), false);
  }
}