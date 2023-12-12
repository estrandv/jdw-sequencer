/*
    Helper functions for MIDI-related calculations. Equations pulled from the internet.
 */

use bigdecimal::{BigDecimal, FromPrimitive};

pub fn beats_to_micro_seconds(beat: f32, bpm: i32) -> i64 {
    (beat * (60.0 / bpm as f32) * 1000000.0) as i64
}

pub fn ms_to_beats(ms: i64, bpm: i32) -> f32 {
    (ms as f32 / 1000.0) / (60.0 / bpm as f32)
}

pub fn ms_to_beats_bd(ms: i64, bpm: i32) -> BigDecimal {
    let microseconds = (BigDecimal::from_i64(ms).unwrap() / BigDecimal::from_i64(1000).unwrap());
    let beats_per_second = BigDecimal::from_i64(60).unwrap() / BigDecimal::from_i32(bpm).unwrap();
    return microseconds / beats_per_second;
}