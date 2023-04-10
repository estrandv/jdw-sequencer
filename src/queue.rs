use std::collections::HashMap;
use chrono::{DateTime, Duration, Utc};
use jdw_osc_lib::TimedOSCPacket;
use log::{debug, info};
use rosc::OscPacket;
use serde::de::Unexpected::Seq;
use crate::{config, midi_utils};

/*
    Machine containing an active sequence and the queued data it will be replaced with once done/shifted.
 */
pub struct Sequencer {
    active_sequence: Vec<TimedOSCPacket>,
    queue: Vec<TimedOSCPacket>,
    current_beat: f32,
    pub end_beat: f32,
    pub started: bool // Used to block immediate execution when a new sequencer is created
}

impl Sequencer {
    pub fn new() -> Self {
        Sequencer {
            active_sequence: vec![],
            queue: vec![],
            current_beat: 0.0,
            end_beat: 0.0,
            started: false
        }
    }


    /*
        TODO: Figuring out the logic puzzle of "started with empty queue"
        - Empty queue call means active sequence will keep runnin but queue is now empty
        - On next shift (active sequence ends vs all sequences end), the empty queue will become active
        - This creates an active sequence that immediately finishes
        - Since this queue acts as nearest neighbour to ANYTHING (including self), any new arrivals will begin immediately
        - More importantly for THIS case: as soon as a new queue arrives, shift will trigger and start the sequence
        - If we set it to unstarted, the active sequence will stop playing immediately
            -> Our current config is RESET_MODE_INDIVIDUAL, which means:
                a. we shift if everything is finished or started
                b. AND we shift everything that is started or finished on individual basis
        - How does it behave when set to !started?
            - Sound immediately cuts
            - Immediately on requeue it is started
        - Why does the sound cut immediately?
            - tick should not consider started, it looks to current beat only
            - If an immediate shift happens, that could explain it
            - I think actually it just... finishes. It's a short sequence.
                -> As such it will be set to finished
                -> Thus it will start immediately on requeue because it is... finished.
        - Why does it immediately requeue?
            - all_finished is false because the other sequence is still running/active
            - shift_finished should only work for finished and started sequences
            - start_all is not triggering (confirmed)
            - Actually this might behave differently when sent in as muted to begin with
                - Shift spam if you mute mid-riff seems to start when the longest sequence finishes
            -

        - Can we just set started=false when shifting into an empty queue?
     */

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

        if !&self.queue.is_empty() && self.started {

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
            info!("Queue shifted. Current beat starts at {}", self.current_beat);
        } else {
            // When queue is empty, delay the shift until an explicit call to started is made.
            // This prevents previously muted queues from immediately shifting into active when requeued.
            // This took a long time to debug and I'm still not 100% sure why it works - could cause more bugs in
            // other run-modes than nearest-neighbour...
            self.started = false;
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

    pub fn start(&mut self) {
        self.started = true;
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
        self.sequences.iter().all(|tuple| tuple.1.is_finished() || !tuple.1.started)
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
            .filter(|seq| seq.1.is_finished() && seq.1.started)
            .map(|seq| seq.1.shift_queue())
            .flatten()
            .collect();

        collected
    }

    // Unstarted sequences (new arrivals) will not reset when finished.
    // This function handles logic for when to start them.
    pub fn start_all_new(&mut self) {
        let amount_unstarted = self.sequences.iter()
            .filter(|seq| !seq.1.started)
            .count();

        let mut should_start = false;
        if amount_unstarted > 0 {
            if config::SEQUENCER_START_MODE == config::SEQ_START_MODE_NEAREST {
                let amount_finished = self.sequences.iter()
                    .filter(|seq| seq.1.started && seq.1.is_finished())
                    .count();

                let amount_started = self.sequences.iter()
                    .filter(|seq| seq.1.started)
                    .count();

                if amount_finished > 0 || amount_started == 0 {
                    debug!("Starting all new sequences because a nearest neighbour is finished.");
                    should_start = true;
                }
            }
            else if config::SEQUENCER_START_MODE == config::SEQ_START_MODE_LONGEST {
                let longest_sequence = self.sequences.iter()
                    .filter(|seq| seq.1.started == true)
                    .max_by(|seq1, seq2| seq1.1.end_beat.total_cmp(&seq2.1.end_beat))
                    .map(|seq| seq.1);

                if longest_sequence.is_some() {
                    if longest_sequence.unwrap().is_finished() {
                        debug!("Starting all new sequences because the longest sequence has finished.");
                        should_start = true;
                    }
                } else {
                    // With no longest (no started) we start immediately
                    debug!("Starting all new sequences because no longest started sequence exists");
                    should_start = true;
                }
            }
            else if config::SEQUENCER_START_MODE == config::SEQ_START_MODE_IMMEDIATE {
                debug!("Starting all new sequences because start mode is set to immediate");
                should_start = true;
            }

        }

        if should_start {
            self.start_all();
        }

    }

    fn start_all(&mut self) {
        self.sequences.iter_mut().for_each(|seq| seq.1.start());
    }

    // Queue a set of timed messages for a given sequencer alias.
    // If no sequencer with the given alias exists, it will be created.
    pub fn queue_sequence(&mut self, alias: &str, new_queue: Vec<TimedOSCPacket>) {


        let existing = self.sequences.get_mut(alias);

        if existing.is_some() {

            existing.map(|seq| {
                seq.set_queue(new_queue);
            });

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