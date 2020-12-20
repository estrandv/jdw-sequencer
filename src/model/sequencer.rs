use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SequencerNote {
    pub tone: f32,
    pub amplitude: f32,
    pub sustain: f32,
    pub start_beat: f32
}