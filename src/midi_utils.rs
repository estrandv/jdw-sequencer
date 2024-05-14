/*
    Helper functions for MIDI-related calculations. Equations pulled from the internet.
 */

use std::str::FromStr;

use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{Duration, TimeDelta};


pub fn beats_to_micro_seconds(beat: f32, bpm: i32) -> i64 {
    (beat * (60.0 / bpm as f32) * 1000000.0) as i64
}

pub fn ms_to_beats(ms: i64, bpm: i32) -> f32 {
    (ms as f32 / 1000.0) / (60.0 / bpm as f32)
}

pub fn duration_to_beats(duration: Duration, bpm: i32) -> BigDecimal {

    // E.g. 60 / 120 = 2 beats per second
    let beats_per_second = BigDecimal::from_i64(60).unwrap() / BigDecimal::from_i32(bpm).unwrap();
    let seconds_elapsed = duration.num_nanoseconds().unwrap() / BigDecimal::from_str("1000000000.000000000").unwrap();
    return seconds_elapsed / beats_per_second;
}