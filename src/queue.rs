
/*
    Shared struct for mutable queue information.
 */
use std::cell::RefCell;
use std::collections::HashMap;
use chrono::{DateTime, Utc,Duration};
use rosc::OscPacket;
use crate::midi_utils;
use serde::{Deserialize, Serialize};



/*
    Message to be executed on relative time for given alias,
    as received from ZeroMQ
 */
#[derive(Debug, Clone)]
pub struct SequencerTickMessage {
    pub alias: String,
    pub time: f32,
    pub msg: OscPacket
}


/*
    Each "alias" will work through its own set of ticks and push its queued set
    into its active sequence when the loop resets.

    [external call] => ApplicationQueue.queue => [main loop]
    => SequenceHandler.queue => [loop reset] => SequenceHandler.active_sequencer

 */
#[derive(Debug)]
pub struct SequenceHandler {
    pub queue: RefCell<UnprocessedSequence>,
    pub active_sequence: RefCell<RealTimeSequence>,
}

/*
 * Self-exhausting set of real time ticks.
 */
#[derive(Debug)]
pub struct RealTimeSequence {
    notes: Vec<RealTimeSequencerTick>,
    pub last_note_time: DateTime<Utc>,
}

impl RealTimeSequence {

    pub fn new_empty() -> Self {
        RealTimeSequence {notes: Vec::new(), last_note_time: chrono::offset::Utc::now()}
    }

    // RestInputNote arrives in relative time format
    // We create a sequence that notes the expected play times in real time units
    // This way note play time is independent from program performance
    pub fn new(notes: Vec<SequencerTickMessage>, start_time: DateTime<Utc>, bpm: i32) -> Self {

        let mut iter_time = start_time.clone();

        let mut sequencer_notes: Vec<RealTimeSequencerTick> = Vec::new();

        for note in notes.iter() {
            sequencer_notes.push(RealTimeSequencerTick {
                message: Option::Some(note.clone()),
                start_time: iter_time.clone()
            });

            let ms = midi_utils::beats_to_micro_seconds(note.clone().time, bpm);
            iter_time = iter_time + Duration::microseconds(ms);

        }

        // To represent the final tone "ringing out" before the next loop starts, we add a final
        // silent note.
        sequencer_notes.push(RealTimeSequencerTick {
            message: Option::None,
            start_time: iter_time.clone()
        });

        let start_time = sequencer_notes.get(0).unwrap().start_time.clone();
        println!("### New loop length: {:?}", iter_time.clone() - start_time);

        RealTimeSequence {
            notes: sequencer_notes, last_note_time: iter_time
        }
    }

    // Pop any notes whose trigger time is lesser or equal to the given current time
    pub fn pop_at_time(&mut self, time: DateTime<Utc>) -> Vec<RealTimeSequencerTick> {
        let candidates = self.notes
            .iter()
            .filter(|n| n.start_time <= time)
            .map(|n| n.clone())
            .collect::<Vec<RealTimeSequencerTick>>();

        self.notes.retain(|n| n.start_time > time);

        candidates
    }

    pub fn is_finished(&self) -> bool {
        self.notes.is_empty()
    }
}


/*
    SequencerTickMessage wrapper parsed in relative order for absolute start-time
 */
#[derive(Debug, Clone)]
pub struct RealTimeSequencerTick {
    pub message: Option<SequencerTickMessage>,
    pub start_time: chrono::DateTime<Utc>,
}


/*
    The pre-realtime format as it appears in the queue
 */
#[derive(Debug, Clone)]
pub struct UnprocessedSequence {
    pub id: String, // Unique id, e.g. "mydrums" - when API queue() is called, this is the id referenced
    pub queue: RefCell<Vec<SequencerTickMessage>>, // Notes to replace the active sequence on next iteration. Changed via API queue() call.
}


pub struct ApplicationQueue {
    pub updated: RefCell<bool>, // Flag; set to true to force the main loop to adopt the new queues
    pub queue: RefCell<Vec<UnprocessedSequence>>, // All separate sequencer queues
}

impl ApplicationQueue {
    pub fn update_queue(&self, payload: Vec<SequencerTickMessage>) {

        // TODO: Tick messages shouldn't have alias contained, but they can (since they will be created
        //  when parsing the queue-bundle, which has the alias)

        let mut grouped_by_alias: HashMap<String, Vec<SequencerTickMessage>> = HashMap::new();
        for msg in payload {
            if !grouped_by_alias.contains_key(&msg.alias) {
                grouped_by_alias.insert(msg.alias.to_string(), Vec::new());
            }
            grouped_by_alias.get_mut(&msg.alias).unwrap().push(msg);
        }

        println!("Queue call received!");
        //println!("Parsed queue message: {:?}", &grouped_by_alias);

        for (alias, value) in grouped_by_alias {

            if value.is_empty() {
                println!("Clearing empty queue data for {}", alias);
            }

            // Clear any pre-existing queue data of that alias
            self.queue.borrow_mut().retain(|e| *e.id != alias);
            //println!("Queueing: {:?} to {}", value.clone(), alias);

            // Create a new queue entry for the alias containing all the notes in the request
            self.queue.borrow_mut().push(UnprocessedSequence {
                id: alias,
                queue: RefCell::new(value)
            });

            // Notify the main thread that queue has been updated
            self.updated.replace(true);
        }
    }
}

mod tests {

    use chrono::Duration;

    use super::{SequencerTickMessage, RealTimeSequence};

    #[test]
    fn sequence_empties() {
        let input = RestInputNote {amplitude: 0.4, sustain_time: 0.3, reserved_time: 0.3, tone: 44.0};

        let mut sequence = RealTimeSequence::new(vec![input], chrono::offset::Utc::now() - Duration::seconds(10), 120);

        sequence.pop_at_time(chrono::offset::Utc::now());

        assert!(sequence.notes.is_empty());
    }

}