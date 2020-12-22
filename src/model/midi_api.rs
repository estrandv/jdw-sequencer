use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MIDIMessage {
    tone: f32,
    sustain_time: f32,
    amplitude: f32
}
impl MIDIMessage { pub fn new(tone: f32, sustain_time: f32, amplitude: f32) -> MIDIMessage { MIDIMessage { tone, sustain_time, amplitude } } }
