use std::cell::RefCell;


use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::midi_utils;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RestInputNote {
    pub tone: f32,
    pub amplitude: f32, 
    pub sustain_time: f32,
    pub reserved_time: f32,
}


#[derive(Debug, Clone, Copy)]
pub struct SequencerNote {
    pub tone: f32,
    pub amplitude: f32,
    pub sustain: f32,
    pub start_time: chrono::DateTime<Utc>,
}

/*
 * Self-exhausting set of sequencer notes. 
 */ 
pub struct Sequence{
    notes: Vec<SequencerNote>,
}

impl Sequence {


    pub fn new_empty() -> Self {
        Sequence {notes: Vec::new()}
    }

    // RestInputNote arrives in relative time format
    // We create a sequence that notes the expected play times in real time units
    // This way note play time is independent from program performance 
    pub fn new(notes: Vec<RestInputNote>, start_time: DateTime<Utc>, bpm: i32) -> Self {
        
        let mut iter_time = start_time.clone();

        let mut sequencer_notes: Vec<SequencerNote> = Vec::new();

        for note in notes.iter() {
            sequencer_notes.push(SequencerNote {
                tone: note.tone.clone(),
                amplitude: note.amplitude.clone(),
                sustain: note.sustain_time.clone(),
                start_time: iter_time.clone()
            });

            let ms = midi_utils::beats_to_milli_seconds(note.reserved_time, bpm);
            iter_time = iter_time + Duration::milliseconds(ms);

            current_beat += note.reserved_time;
        }

        // To represent the final tone "ringing out" before the next loop starts, we add a final
        // silent note. 
        sequencer_notes.push(SequencerNote {
                tone: 0.0,
                amplitude: 0.0,
                sustain: 0.0,
                start_time: iter_time.clone()
        });


        Sequence {
           notes: sequencer_notes 
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
    pub queue: RefCell<Vec<RestInputNote>>, // Notes to replace the active sequence on next iteration. Changed via API queue() call. 
}

pub struct SequencerMetaData {
    pub queue: RefCell<SequencerQueueData>, 
    pub active_sequence: RefCell<Sequence>,
}

mod tests {

    use chrono::Duration;

    use super::{RestInputNote, Sequence};


    #[test]
    fn sequence_empties() {
        let input = RestInputNote {amplitude: 0.4, sustain_time: 0.3, reserved_time: 0.3, tone: 44.0};

        let mut sequence = Sequence::new(vec![input], chrono::offset::Utc::now() - Duration::seconds(10), 120);

        sequence.pop_at_time(chrono::offset::Utc::now());

        assert!(sequence.notes.is_empty());
    }

} 
