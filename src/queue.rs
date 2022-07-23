use std::collections::HashMap;
use chrono::{DateTime, Duration, Utc};
use rosc::OscPacket;
use serde::de::Unexpected::Seq;
use crate::{midi_utils, TimedOscMessage};


/*
    Packet to be sent at a certain time within a sequence.
 */
pub struct RealTimePacket {
    pub packet: Option<OscPacket>,
    pub time: DateTime<Utc>,
}

/*
    Self-emptying set of real time packets - populated on new() and then emptied using pop_on_time until
    nothing remains. Typically thrown straight in trash and replaced after this.
 */
pub struct RealTimePacketSequence {
    timed_packets: Vec<RealTimePacket>,
}

impl RealTimePacketSequence {

    pub fn empty() -> RealTimePacketSequence {
        RealTimePacketSequence {timed_packets: vec![]}
    }

    // Add actual calculated execution time for each message and use that vector to construct a sequence
    pub fn new(messages: Vec<TimedOscMessage>, start_time: DateTime<Utc>, bpm: i32) -> RealTimePacketSequence {

        let mut iter_time = start_time.clone();
        let mut real_time_packets : Vec<RealTimePacket> = Vec::new();

        for message in messages {
            let packet = OscPacket::Message(message.message);
            let wrapped_packet = RealTimePacket {
                packet: Some(packet),
                time: iter_time.clone()
            };

            real_time_packets.push(wrapped_packet);

            let ms = midi_utils::beats_to_micro_seconds(message.time, bpm);
            iter_time = iter_time + Duration::microseconds(ms);
        }

        // Each sequence will only reset once all packets have reached their time
        // We thus add a final dummy tick to let the real last message keep its padding time
        let final_tick = RealTimePacket {
            packet: None,
            time: iter_time.clone()
        };
        real_time_packets.push(final_tick);

        RealTimePacketSequence {
            timed_packets: real_time_packets
        }

    }


    // Remove and receive the contained packets of any timed packets whose trigger time is
    // lesser or equal to the given current time
    pub fn pop_at_time(&mut self, time: &DateTime<Utc>) -> Vec<OscPacket> {
        let candidates = self.timed_packets
            .iter()
            .filter(|n| &n.time <= time)
            .map(|n| n.clone().packet.clone())
            .filter(|n| n.is_some())
            .map(|o| o.unwrap())
            .collect();

        self.timed_packets.retain(|n| &n.time > time);

        candidates
    }

    pub fn is_finished(&self) -> bool {
        self.timed_packets.is_empty()
    }

    pub fn get_end_time(&self) -> DateTime<Utc> {
        if self.timed_packets.is_empty() {
            return Utc::now();
        }

        self.timed_packets.iter().last().unwrap().time.clone()
    }

}

/*
    Machine containing an active sequence and the queued data it will be replaced with once done/shifted.
 */
pub struct Sequencer {
    active_sequence: RealTimePacketSequence,
    queue: Vec<TimedOscMessage>
}

impl Sequencer {

    pub fn new() -> Self {
        Sequencer {
            active_sequence: RealTimePacketSequence::empty(),
            queue: vec![]
        }
    }

    // Replace active sequence with the queued one
    pub fn shift_queue(&mut self, start_time: &DateTime<Utc>, bpm: i32) {
        if !self.queue.is_empty() {
            self.active_sequence = RealTimePacketSequence::new(
                self.queue.clone(),
                start_time.clone(),
                bpm
            );
        }
    }

    pub fn set_queue(&mut self, new_queue: Vec<TimedOscMessage>) {
        self.queue = new_queue;
    }

}

// Registry of all active sequencers and their aliases
pub struct SequencerHandler {
    sequences: HashMap<String, Sequencer>
}

impl SequencerHandler {

    pub fn new() -> SequencerHandler {
        SequencerHandler { sequences: HashMap::new()}
    }

    pub fn is_empty(&self) -> bool {
        self.sequences.is_empty()
    }

    pub fn all_sequences_finished(&self) -> bool {
        // If there are no messages left to send for any sequence (all popped; all times passed)
        self.sequences.iter().all(|tuple| tuple.1.active_sequence.is_finished())
    }

    pub fn empty_all(&mut self) {
        self.sequences = HashMap::new();
    }

    // Determine a start time for the next loop and use that to shift all contained sequencer queues
    // (=start a new loop)
    pub fn shift_queues(&mut self, current_bpm: i32, this_loop_time: &DateTime<Utc>) {

        // We cannot rely on the current tick time to supply a new start time, since
        // it might overshoot the final note time by some amount of microseconds.
        // Instead we should find what the latest note time was and start from there.

        let longest_sequence = self.sequences.iter()
            .max_by_key(|seq| seq.1.active_sequence.get_end_time());

        // Last note time is new start time
        let new_loop_start_time = match longest_sequence {
            Some(seq) => seq.1.active_sequence.get_end_time(),
            None => {
                log::debug!("No max time found, using that of current loop.");
                this_loop_time.clone()
            }
        };

        for data in self.sequences.iter_mut() {
            data.1.shift_queue(&new_loop_start_time, current_bpm);
        }

        let longest_next = self.sequences.iter()
            .max_by_key(|seq| seq.1.active_sequence.get_end_time());

        let last_next_loop_note_time = match longest_next {
            Some(seq) => seq.1.active_sequence.get_end_time(),
            None => this_loop_time.clone()
        };

        // TODO: Was conditional on queue: !self.queue_data.lock().unwrap().queue.borrow().is_empty()
        // Not that this should happen in here anyway. ....
        if false {
            log::info!(
                        "Starting a new loop at time: {}, new loop start time: {}, end time: {}",
                        chrono::offset::Utc::now(),
                        new_loop_start_time,
                        last_next_loop_note_time
                    );
        }

        // TODO: Loop start out msg is posted here
    }

    // Queue a set of timed messages for a given sequencer alias.
    // If no sequencer with the given alias exists, it will be created.
    pub fn queue_sequence(&mut self, alias: &str, new_queue: Vec<TimedOscMessage>) {
        let existing = self.sequences.iter_mut()
            .find(|data| &data.0.clone() == alias)
            .map(|tuple| tuple.1);

        if existing.is_some() {
            existing.unwrap().set_queue(new_queue);
        } else {
            let mut new_seq = Sequencer::new();
            new_seq.set_queue(new_queue);
            self.sequences.insert(alias.to_string(), new_seq);
        }

    }

    // Pop all messages that match the given time from all contained sequencers, returning them as a combined vector
    pub fn pop_on_time(&mut self, time: &DateTime<Utc>) -> Vec<OscPacket> {

        let mut all_messages: Vec<OscPacket> = Vec::new();

        // Find messages matching the current time
        for meta_data in self.sequences.iter_mut() {
            let on_time = meta_data.1.active_sequence.pop_at_time(time);

            if !on_time.is_empty() {

                // Post the messages to the out socket
                {
                    let unwrapped: Vec<_> = on_time.iter()
                        .map(|p| p.clone())
                        .collect();
                    all_messages.extend(unwrapped);
                }
            }
        }

        return all_messages;
    }
}

// TODO
mod tests {

    #[test]
    fn sequence_empties() {

    }

}