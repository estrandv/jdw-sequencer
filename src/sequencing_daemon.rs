use std::{cell::RefCell, str::FromStr, sync::Arc, thread};

use bigdecimal::{BigDecimal};
use chrono::{DateTime, Utc, Duration};
use jdw_osc_lib::model::TimedOSCPacket;
use log::{info, warn, debug};
use ringbuf::{storage::Heap, traits::Consumer, wrap::caching::Caching, SharedRb};
use rosc::OscPacket;

use crate::{local_messaging::LocalSequencerMessage, master_sequencer::MasterSequencer, midi_utils, sequencer::SequencerEntry};

/*
    Below struct and function are a rewrite of logic previously contained in queue.rs::shift_queue. 
    They should live somewhere else, determined by usage. 
*/

pub struct OscSequencePayload {
    pub message_sequence: Vec<SequencerEntry<OscPacket>>,
    pub end_beat: BigDecimal
}

pub fn to_sequence(input: Vec<TimedOSCPacket>) -> OscSequencePayload {


    let mut new_sequence: Vec<SequencerEntry<OscPacket>> = vec![];
    let mut new_timeline = BigDecimal::from_str("0.0").unwrap();

    for packet in &input {
        new_sequence.push(SequencerEntry::new(new_timeline.clone(), packet.packet.clone()));

        new_timeline += packet.time.clone();
    }

    // TODO: Note the composite payload - sequencer.rs takes an end_beat for queue
    OscSequencePayload {
        message_sequence: new_sequence,
        end_beat: new_timeline
    }

}



pub struct SequencingDaemonState {
    pub bpm: RefCell<i32>,
    pub reset: RefCell<bool>,
    pub hard_stop: RefCell<bool>
}

impl SequencingDaemonState {
    pub fn new(bpm_param: i32) -> SequencingDaemonState {
        SequencingDaemonState { bpm: RefCell::new(bpm_param), reset: RefCell::new(false), hard_stop: RefCell::new(false) }
    }
}

pub fn start_live_loop <T: 'static + Clone + Send, F> (
    mut master_sequencer: MasterSequencer<T>,
    bpm_param: i32,
    mut message_sub: Caching<Arc<SharedRb<Heap<LocalSequencerMessage<T>>>>, false, true>,
    entry_operations: F
) where F: 'static + Send + Fn(Vec<T>) -> () {
    thread::spawn(move || {

        let state = SequencingDaemonState::new(bpm_param);

        let mut last_loop_time: Option<DateTime<Utc>> = None;

        let sleeper = spin_sleep::SpinSleeper::new(100);
        
        loop {
            /*
                Calculate and log loop time taken 
            */
            let this_loop_time = Utc::now();
            let elapsed_time = match last_loop_time {
                Some(t) => {
                    this_loop_time.time() - t.time()
                }
                None => Duration::zero()
            };
            last_loop_time = Some(this_loop_time.clone());
            
            //info!("New master loop began - time taken since last loop (microsec): {:?}", elapsed_time.num_microseconds());

            /*
                Read input previously written to state via OSC 
            */

            let current_bpm = state.bpm.clone();
            let reset_requested = state.reset.clone().into_inner();
            let hard_stop_requested = state.hard_stop.clone().into_inner();
            {
                state.reset.replace(false);
                state.hard_stop.replace(false);
            }

            let elapsed_beats = midi_utils::duration_to_beats(elapsed_time, current_bpm.clone().into_inner());

            //info!("Elapsed beats: {}", elapsed_beats);

            /*
                TODO: Stop request 
                - A stop means "Reset all sequencers and treat them as not started until a new start is received"
                - Then, of course, there is also the FULL STOP, where all sequencers are simply eliminated 
                    -> This is what we do below, there is no regular stop currently! 

                TODO TODO: Do we really need a state? 
                    - I think some initial idea was that e.g. the wipe or force_reset should happen at a particular place
                    - This can probably be handled with closures if you know how to write them, but I'm also kinda sure 
                        that most things can just happen immediately 
            */
            if hard_stop_requested {
                master_sequencer.force_wipe();
            } else {
                /*
                    Tick the clock, collect Ts on time. 
                */
                master_sequencer.start_check();
                
                if reset_requested {
                    master_sequencer.force_reset();
                } else {
                    master_sequencer.reset_check();
                }
                let collected = master_sequencer.tick(elapsed_beats).clone(); // TODO: Does it work without clone, or does that make lock eternal? 
                
                entry_operations(collected);
            }


            /*

                Incoming state updates

            */

            // TODO: This new method renders a lot of old designs obsolete (e.g. this initial writing before later reading of state)
            // TODO: Might want to limit how many messages we read at a time 
            while let Some(msg) = message_sub.try_pop() {

                print!("POP");

                match msg {
                    LocalSequencerMessage::HardStop => {state.hard_stop.replace(true);},
                    LocalSequencerMessage::Reset => {state.reset.replace(true);},
                    LocalSequencerMessage::SetBpm(new_bpm) => {state.bpm.replace(new_bpm);},
                    LocalSequencerMessage::EndAfterFinish => {master_sequencer.end_after_finish();},
                    LocalSequencerMessage::Queue(payload) => {
                        info!("QUEUE RECEIVED");
                        master_sequencer.queue(
                            payload.sequencer_alias.as_str(),
                             payload.entries, 
                             payload.end_beat, 
                             payload.one_shot
                            );
                    },
                    // TODO: See handling of osc message, this isn't doing anything atm
                    LocalSequencerMessage::BatchQueue(payloads) => {
                        for payload in payloads {
                            master_sequencer.queue(
                                payload.sequencer_alias.as_str(),
                                 payload.entries, 
                                 payload.end_beat, 
                                 payload.one_shot
                                );
                        }
                    },
                }
            }


            /*
                Calculate time taken to execute this loop and log accordingly
            */
            let dur = Utc::now().time() - this_loop_time.time();
            let time_taken_ns = dur.num_nanoseconds().expect(
                "Failed to resolve loop time as nanoseconds - is it too large to fit an i64?"
            ) as u64; // Make it crash if unwrap fails - there is no good alternative to the real number, if subtracting from tick time!

            let tick_time_ns = crate::config::TICK_TIME_US * 1000;

            if time_taken_ns > tick_time_ns {
                warn!("Operations performed (time: {}) exceed tick time, overflow...", time_taken_ns);
            }
            /*
                Sleep until next loop tick
            */
            debug!("End loop: {}", this_loop_time);

            /*
                TODO.
                Running some tests here.
                Constant tick time appears to be more stable than the diffcheck, even though the
                    diffcheck makes intuitive sense.
                UPDATE: Constant tick time is -not- more stable when accounting for total drift, only appears that way for individual drift.
                UPDATE: Began using NANOS instead of micros, which gave me (in combination with dynamic tick time) at least one example
                    of long time (>6min) stable drift.
                    - Nanos for elapsed beats calculation is perfect
                    - Nanos for relative tick time (instead of previous micros) did not seem to improve things much if at all
                        (but I don't see how it could be worse!).
             */

            //sleeper.sleep(std::time::Duration::from_micros(crate::config::TICK_TIME_US));
            let time_left_until_tick = if time_taken_ns < tick_time_ns { tick_time_ns - time_taken_ns } else {0};
            sleeper.sleep(std::time::Duration::from_nanos(time_left_until_tick));

        }


    });
}