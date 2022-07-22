
/*
    Shared struct for mutable queue information.
 */
use std::cell::RefCell;
use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use log::{debug, info};
use rosc::OscPacket;

use crate::midi_utils;

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

    // Note how relative time is converted to real time here
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
        debug!("### New loop length: {:?}", iter_time.clone() - start_time);

        RealTimeSequence {
            notes: sequencer_notes, last_note_time: iter_time
        }
    }

    // Pop any notes whose trigger time is lesser or equal to the given current time
    pub fn pop_at_time(&mut self, time: &DateTime<Utc>) -> Vec<RealTimeSequencerTick> {
        let candidates = self.notes
            .iter()
            .filter(|n| &n.start_time <= time)
            .map(|n| n.clone())
            .collect::<Vec<RealTimeSequencerTick>>();

        self.notes.retain(|n| &n.start_time > time);

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
    pub alias: String, // Unique id, e.g. "mydrums" - when API queue() is called, this is the id referenced
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

        info!("Queue call received!");

        for (alias, value) in grouped_by_alias {

            info!("Alias {} updating ...", &alias);

            if value.is_empty() {
                info!("Clearing queue data for {}", alias);
            }

            // Clear any pre-existing queue data of that alias
            self.queue.borrow_mut().retain(|e| *e.alias != alias);

            // Create a new queue entry for the alias containing all the notes in the request
            self.queue.borrow_mut().push(UnprocessedSequence {
                alias: alias,
                queue: RefCell::new(value)
            });

            // Notify the main thread that queue has been updated
            self.updated.replace(true);
        }
    }
}

pub struct MasterHandler {
    sequence_handlers: Vec<SequenceHandler>
}

impl MasterHandler {

    pub fn new() -> MasterHandler {
        MasterHandler {sequence_handlers: vec![]}
    }

    pub fn is_empty(&self) -> bool {
        self.sequence_handlers.is_empty()
    }

    pub fn all_sequences_finished(&self) -> bool {
        // If there are no notes left to play, reset the sequencer by pushing queues into state
        self.sequence_handlers.iter().all(|data| data.active_sequence.borrow().is_finished())
    }

    pub fn empty_all(&mut self) {
        self.sequence_handlers = Vec::new();
    }

    pub fn shift_queues(&mut self, current_bpm: i32, this_loop_time: &DateTime<Utc>) {
        // TODO: vars: this_loop_time, current_bpm

        // We cannot rely on the current tick time to supply a new start time, since
        // it might overshoot the final note time by some amount of microseconds.
        // Instead we should find what the latest note time was and start from there.

        let longest_sequence = self.sequence_handlers.iter()
            .max_by_key(|seq| seq.active_sequence.borrow().last_note_time);

        // Last note time is new start time
        let new_loop_start_time = match longest_sequence {
            Some(seq) => seq.active_sequence.borrow().last_note_time,
            None => {
                debug!("No max time found, using that of current loop.");
                this_loop_time.clone()
            }
        };

        for data in self.sequence_handlers.iter() {
            if !data.queue.borrow().queue.borrow().is_empty() {
                data.active_sequence.replace(RealTimeSequence::new(
                    data.queue.borrow().queue.clone().into_inner(),
                    new_loop_start_time,
                    current_bpm.clone())
                );
            }
        }

        let longest_next = self.sequence_handlers.iter()
            .max_by_key(|seq| seq.active_sequence.borrow().last_note_time);

        let last_next_loop_note_time = match longest_next {
            Some(seq) => seq.active_sequence.borrow().last_note_time,
            None => this_loop_time.clone()
        };

        // TODO: Was conditional on queue: !self.queue_data.lock().unwrap().queue.borrow().is_empty()
        // Not that this should happen in here anyway. ....
        if true {
            info!(
                        "Starting a new loop at time: {}, new loop start time: {}, end time: {}",
                        chrono::offset::Utc::now(),
                        new_loop_start_time,
                        last_next_loop_note_time
                    );
        }

        // TODO: Loop start out msg is posted here
    }

    // Use new_queues to replace the "coming up next" message sequence in each sequencer.
    // All sequensers with an alias matching a new queue will be replaced.
    // TODO: API might as well call this directly and skip the whole "was updated" bit completely
    pub fn replace_queues(&mut self, new_queues: Vec<UnprocessedSequence>) {
        for queue in new_queues {
            let existing = self.sequence_handlers.iter().find(|data| data.queue.borrow().alias == queue.alias);

            // If a queue with the same id exists, we change the queue data according to
            // request. If not, we create new queue data with an empty sequence to be
            // populated the next time the queue replaces current.
            match existing {
                Some(old_data) => {
                    old_data.queue.replace(queue.clone());
                }
                None => {
                    let new_data = SequenceHandler {
                        queue: RefCell::new(queue.clone()),
                        active_sequence: RefCell::new(RealTimeSequence::new_empty()),
                    };

                    self.sequence_handlers.push(new_data);
                }
            }
        }
    }

    // Pop all messages that match the given time from all contained sequencers, returning them as a combined vector
    pub fn pop_on_time(&mut self, time: &DateTime<Utc>) -> Vec<OscPacket> {

        let mut all_messages: Vec<OscPacket> = Vec::new();

        // Find messages matching the current time
        for meta_data in self.sequence_handlers.iter_mut() {
            let on_time = meta_data.active_sequence.get_mut().pop_at_time(time);

            if !on_time.is_empty() {

                // Post the messages to the out socket
                {
                    let unwrapped: Vec<_> = on_time.iter()
                        .filter(|t| t.message.clone().is_some())
                        .map(|t| t.message.clone().unwrap().msg)
                        .collect();

                    for packet in unwrapped {
                        all_messages.push(packet);
                    }
                }
            }
        }

        return all_messages;
    }
}

mod tests {
    use chrono::Duration;
    use rosc::{OscMessage, OscPacket};
    use crate::{RealTimeSequence, SequencerTickMessage};

    #[test]
    fn sequence_empties() {

        let msg = SequencerTickMessage {
            alias: "test".to_string(),
            time: 0.0,
            msg: OscPacket::Message(OscMessage::from("/msg"))
        };

        let mut sequence = RealTimeSequence::new(vec![msg], chrono::offset::Utc::now() - Duration::seconds(10), 120);

        sequence.pop_at_time(chrono::offset::Utc::now());

        assert!(sequence.notes.is_empty());
    }

}