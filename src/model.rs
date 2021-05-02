use std::cell::RefCell;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{midi_utils};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequencerNoteMessage {
    pub target: String,
    pub alias: String,
    pub time: f32,
    pub args: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MIDINotePlayMessage {
    pub(crate) target: String,
    pub(crate) tone: i32,
    pub(crate) sus_ms: f32,
    pub (crate) amp: f32,
}

impl SequencerNoteMessage {

    // TODO: Getting kinda clumsy
    pub fn get(self, key: &str) -> Option<f32> {
        if self.args.contains_key(key) {
            Option::Some(self.args.get(key).unwrap().clone())
        } else {
            Option::None
        }
    }
}

#[derive(Debug, Clone)]
pub struct SequencerNote {
    pub message: Option<SequencerNoteMessage>,
    pub start_time: chrono::DateTime<Utc>,
}

impl SequencerNote {
    pub fn get_attr(self, key: &str) -> Option<f32> {
        match self.message {
            Some(m) => m.get(key),
            None => Option::None
        }
    }

    pub fn convert(&self) -> SequencerNoteMessage {
        self.message.clone().unwrap() // TODO: Dangerous optional, also freq vs tone
    }
}

/*
 * Self-exhausting set of sequencer notes. 
 */ 
pub struct Sequence{
    notes: Vec<SequencerNote>,
    pub last_note_time: DateTime<Utc>,
}

impl Sequence {

    pub fn new_empty() -> Self {
        Sequence {notes: Vec::new(), last_note_time: chrono::offset::Utc::now()}
    }

    // RestInputNote arrives in relative time format
    // We create a sequence that notes the expected play times in real time units
    // This way note play time is independent from program performance 
    pub fn new(notes: Vec<SequencerNoteMessage>, start_time: DateTime<Utc>, bpm: i32) -> Self {
        
        let mut iter_time = start_time.clone();

        let mut sequencer_notes: Vec<SequencerNote> = Vec::new();

        for note in notes.iter() {
            sequencer_notes.push(SequencerNote {
                message: Option::Some(note.clone()),
                start_time: iter_time.clone()
            });

            let ms = midi_utils::beats_to_micro_seconds(note.clone().time, bpm);
            iter_time = iter_time + Duration::microseconds(ms);

        }

        // To represent the final tone "ringing out" before the next loop starts, we add a final
        // silent note.
        sequencer_notes.push(SequencerNote {
                message: Option::None,
                start_time: iter_time.clone()
        });

        Sequence {
           notes: sequencer_notes,
            last_note_time: iter_time
        }
    }

    // Pop any notes whose trigger time is lesser or equal to the given current time
    pub fn pop_at_time(&mut self, time: DateTime<Utc>) -> Vec<SequencerNote> {
        let candidates = self.notes
            .iter()
            .filter(|n| n.start_time <= time)
            .map(|n| n.clone())
            .collect::<Vec<SequencerNote>>();

        self.notes.retain(|n| n.start_time > time);

        candidates
    }

    pub fn is_finished(&self) -> bool {
        self.notes.is_empty()
    }
}

#[derive(Debug, Clone)]
pub enum OutputTargetType {
    Prosc, 
    ProscSample,
    MIDI,
}

#[derive(Debug, Clone)]
pub struct SequencerQueueData {
    pub id: String, // Unique id, e.g. "mydrums" - when API queue() is called, this is the id referenced 
    pub target_type: OutputTargetType, // Where the sequence plays to. Determines what rest endpoint is called when playing notes. 
    pub instrument_id: String, // Id used to identify instrument at target_type 
    pub queue: RefCell<Vec<SequencerNoteMessage>>, // Notes to replace the active sequence on next iteration. Changed via API queue() call.
}

pub struct QueueMetaData {
    pub updated: RefCell<bool>,
    pub queue: RefCell<Vec<SequencerQueueData>>,
}

pub struct SequencerMetaData {
    pub queue: RefCell<SequencerQueueData>, 
    pub active_sequence: RefCell<Sequence>,
}

mod tests {

    use chrono::Duration;

    use super::{SequencerNoteMessage, Sequence};


    #[test]
    fn sequence_empties() {
        let input = RestInputNote {amplitude: 0.4, sustain_time: 0.3, reserved_time: 0.3, tone: 44.0};

        let mut sequence = Sequence::new(vec![input], chrono::offset::Utc::now() - Duration::seconds(10), 120);

        sequence.pop_at_time(chrono::offset::Utc::now());

        assert!(sequence.notes.is_empty());
    }

} 
