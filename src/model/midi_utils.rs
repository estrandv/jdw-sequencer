
/*
    Helper functions for MIDI-related calculations. Equations pulled from the internet.
 */

pub fn beats_to_milli_seconds(beat: f32, bpm: i32) -> i64 {
    (beat * (60.0 / bpm as f32) * 1000.0) as i64
}

// TODO: Reversed from function above. Anton suggested it might be Beat = (ms/1000) / (60/bpm)
pub fn ms_to_beats(ms: i64, bpm: i32) -> f32 {
    (ms as f32 / 1000.0) / (60.0 / bpm as f32)
}

pub fn ms_to_beats_old(ms: i64, bpm: i32) -> f32 {
    ((ms as f32 / 1000.0) * 60.0) / bpm as f32
}