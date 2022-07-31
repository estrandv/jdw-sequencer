use std::collections::HashMap;
use chrono::{DateTime, Duration, Utc};
use jdw_osc_lib::TimedOSCPacket;
use log::info;
use rosc::OscPacket;
use serde::de::Unexpected::Seq;
use crate::{midi_utils};


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
        RealTimePacketSequence { timed_packets: vec![] }
    }

    // Add actual calculated execution time for each message and use that vector to construct a sequence
    pub fn new(messages: Vec<TimedOSCPacket>, start_time: DateTime<Utc>, bpm: i32) -> RealTimePacketSequence {
        let mut iter_time = start_time.clone();
        let mut real_time_packets: Vec<RealTimePacket> = Vec::new();

        for message in messages {
            let wrapped_packet = RealTimePacket {
                packet: Some(message.packet),
                time: iter_time.clone(),
            };

            real_time_packets.push(wrapped_packet);

            let ms = midi_utils::beats_to_micro_seconds(message.time, bpm);
            iter_time = iter_time + Duration::microseconds(ms);
        }

        // Each sequence will only reset once all packets have reached their time
        // We thus add a final dummy tick to let the real last message keep its padding time
        let final_tick = RealTimePacket {
            packet: None,
            time: iter_time.clone(),
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
    TODO: WIll take on much of RealTimePacketSequence, remove that later
 */
pub struct Sequencer {
    active_sequence: Vec<TimedOSCPacket>,
    queue: Vec<TimedOSCPacket>,
    current_beat: f32,
    end_beat: f32,
}

impl Sequencer {
    pub fn new() -> Self {
        Sequencer {
            active_sequence: vec![],
            queue: vec![],
            current_beat: 0.0,
            end_beat: 0.0
        }
    }

    // Add elapsed time to current_beat, pop any passed messages, and return them
    pub fn tick_and_return(&mut self, elapsed_beats: &f32) -> Vec<OscPacket> {

        // If we keep ticking the time after finishing, we will build up
        // an overshoot big enough to skip the whole next loop. So we don't.
        if !&self.is_finished() {

            self.current_beat += elapsed_beats;
            let candidates: Vec<_> = self.active_sequence
                .iter()
                .filter(|n| &n.time <= &self.current_beat)
                .map(|n| n.clone().packet.clone())
                .collect();

            let beat = self.current_beat;
            self.active_sequence.retain(|seq| &seq.time > &beat);

            if !&candidates.is_empty() {
                info!("Tick time is now {} and {} candidates were found. Remaining: {}.", self.current_beat, candidates.len(), &self.active_sequence.len());
            }

            return candidates;
        }

        return vec![];

    }

    // Replace active sequence with the queued one
    // Return any times we've already passed (e.g. messages with time 0.0)
    pub fn shift_queue(&mut self) -> Vec<OscPacket> {
        let last_end_time = self.end_beat;

        if !&self.queue.is_empty() {

            let mut new_sequence: Vec<TimedOSCPacket> = vec![];
            let mut new_timeline: f32 = 0.0;

            // Note how the only difference is that the active sequence has incremental, relative time
            // for each note.
            for packet in &self.queue {
                new_sequence.push(TimedOSCPacket {
                    time: new_timeline,
                    packet: packet.packet.clone()
                });
                new_timeline += packet.time;
            }

            // TODO: Always tricky with shifts - be mindful of cases like
            // last tick overshooting the end - do we do carryover or simply save it when
            // passing next elapsed_beats?

            // Here's how to carry over, either way
            let overshoot = if self.current_beat > last_end_time && last_end_time > 0.0 {
                self.current_beat - last_end_time
            } else {0.0};

            self.current_beat = overshoot;
            self.active_sequence = new_sequence;
            self.end_beat = new_timeline; // Includes the "ring out" time of the last packet
            info!("Current beat starts at {}", self.current_beat);
        }

        // Since we now are at least on 0.0 (possibly even on a bit of overshoot), we should get
        // any early messages with this even though elapsed_beats is zero.
        self.tick_and_return(&0.0)
    }


    pub fn is_finished(&self) -> bool {
        self.current_beat >= self.end_beat
    }

    pub fn set_queue(&mut self, new_queue: Vec<TimedOSCPacket>) {
        self.queue = new_queue;
    }
}

// Registry of all active sequencers and their aliases
pub struct SequencerHandler {
    sequences: HashMap<String, Sequencer>,
}

impl SequencerHandler {
    pub fn new() -> SequencerHandler {
        SequencerHandler { sequences: HashMap::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.sequences.is_empty()
    }

    pub fn all_sequences_finished(&self) -> bool {
        // If there are no messages left to send for any sequence (all popped; all times passed)
        self.sequences.iter().all(|tuple| tuple.1.is_finished())
    }

    pub fn empty_all(&mut self) {
        self.sequences = HashMap::new();
    }

    // Call shift_queues on all contained sequences, returning a combined result of packets
    pub fn shift_queues(&mut self) -> Vec<OscPacket> {

        // TODO: Potential bug here too - what if shift happens later than the last tick,
        // how does that handle overshoot? Might be that "tick again after shift" solves that
        // but bear in mind! See the long rant in the main loop.

        let collected = self.sequences.iter_mut()
            .map(|seq| seq.1.shift_queue())
            .flatten()
            .collect();

        collected
    }

    // Like the above, but dynamically shifting any sequencer that has finished instead of
    // manually force-shifting everything.
    // Useful if sequencers are running in individual mode.
    pub fn shift_finished(&mut self) -> Vec<OscPacket> {

        let collected = self.sequences.iter_mut()
            .filter(|seq| seq.1.is_finished())
            .map(|seq| seq.1.shift_queue())
            .flatten()
            .collect();

        collected
    }

    // Queue a set of timed messages for a given sequencer alias.
    // If no sequencer with the given alias exists, it will be created.
    pub fn queue_sequence(&mut self, alias: &str, new_queue: Vec<TimedOSCPacket>) {
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
    pub fn tick_and_return_all(&mut self, elapsed_beats: &f32) -> Vec<OscPacket> {
        let mut all_messages: Vec<OscPacket> = Vec::new();

        // Find messages matching the current time
        for meta_data in self.sequences.iter_mut() {
            let on_time = meta_data.1.tick_and_return(elapsed_beats);

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
    fn sequence_empties() {}
}