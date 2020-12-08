use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SequencerNote {
    pub tone: i32,
    pub amplitude: f32,
    pub sustain: f32,
    pub startBeat: f32
}