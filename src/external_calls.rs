use crate::model::{SequencerNote, SequencerNoteMessage};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct  MIDIMessage {
    tone: f32,
    sustain_time: f32, 
    amplitude: f32,
}

