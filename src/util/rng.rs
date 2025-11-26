//! Thread-local seeded RNG

use std::cell::RefCell;

use rand::{Rng, SeedableRng, rngs::SmallRng};

thread_local! {
    static RNG: RefCell<SmallRng> = RefCell::new(SmallRng::seed_from_u64(0));
}

/// Seeds the thread-local RNG
pub fn seed(s: u64) {
    RNG.with(|rng| *rng.borrow_mut() = SmallRng::seed_from_u64(s));
}

/// Returns a uniformly distributed `f32` in `[low, high]`
pub fn random_range(low: f32, high: f32) -> f32 {
    RNG.with(|rng| rng.borrow_mut().random_range(low..=high))
}

/// Returns `true` with probability `p` in `[0.0, 1.0]`
pub fn random_bool(p: f64) -> bool {
    RNG.with(|rng| rng.borrow_mut().random_bool(p))
}
