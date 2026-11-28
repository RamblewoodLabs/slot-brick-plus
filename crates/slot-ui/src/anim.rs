//! One speed for every wall-clock animation in Slot.
//!
//! Authored timings (the numbers in the code and the docs) stay as they were written;
//! they are played at [`ANIM_SPEED`]. 1.4 is a 40% speed-up: durations are 1/1.4 ≈ 0.714
//! of the authored length, spring frequencies 1.4 times higher.

/// Wall-clock animations play this many times faster than their authored timings.
pub const ANIM_SPEED: f32 = 1.4;

/// Authored seconds, played at [`ANIM_SPEED`].
pub const fn paced_s(seconds: f32) -> f32 {
    seconds / ANIM_SPEED
}

/// Authored milliseconds, played at [`ANIM_SPEED`].
pub const fn paced_ms(ms: u64) -> u64 {
    (ms as f32 / ANIM_SPEED + 0.5) as u64
}
