/*

    Rewrite of... Idunno, main loop? But with the new sequencer/master sequencer classes. 
    Should just tick based on bpm in a thread. 


    TODO: 
        - Clean up old main loop by extracting incoming osc handling
            -> somewhat done! 
        - Make a method for converting incoming osc to timeline-adjusted entries in osc packets 
            -> See below, working on it here but no usage yet 
        - re-implement the start/stop/reset logic from old main 
        - Backup old main, then use a new copy to implement this daemon in place of the old main loop 

*/

use std::{sync::{Arc, Mutex}, thread, str::FromStr, cell::RefCell};

use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, Utc, Duration};
use jdw_osc_lib::TimedOSCPacket;
use log::{info, warn, debug};
use rosc::OscPacket;

use crate::{master_sequencer::MasterSequencer, midi_utils, sequencer::SequencerEntry};

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

    info!("END TIME WAS: {:?}", new_timeline);

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
    master_sequencer: Arc<Mutex<MasterSequencer<T>>>,
    state: Arc<Mutex<SequencingDaemonState>>,
    entry_operations: F
) where F: 'static + Send + Fn(Vec<T>) -> () {
    thread::spawn(move || {

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

            let current_bpm = state.lock().unwrap().bpm.clone();
            let reset_requested = state.lock().unwrap().reset.clone().into_inner();
            let hard_stop_requested = state.lock().unwrap().hard_stop.clone().into_inner();
            {
                state.lock().unwrap().reset.replace(false);
                state.lock().unwrap().hard_stop.replace(false);
            }            
            
            let elapsed_microsec = BigDecimal::from_i64(elapsed_time.num_microseconds().unwrap()).unwrap();
            let elapsed_beats = midi_utils::mcs_to_beats_bd(elapsed_microsec, current_bpm.clone().into_inner());

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
                master_sequencer.lock().unwrap().force_wipe();
            } else {
                /*
                    Tick the clock, collect Ts on time. 
                */
                master_sequencer.lock().unwrap().start_check();
                
                if reset_requested {
                    master_sequencer.lock().unwrap().force_reset();
                } else {
                    master_sequencer.lock().unwrap().reset_check();
                }
                let collected = master_sequencer.lock().unwrap().tick(elapsed_beats).clone(); // TODO: Does it work without clone, or does that make lock eternal? 
                
                entry_operations(collected);
            }


            /*
                Calculate time taken to execute this loop and log accordingly
            */
            let dur = Utc::now().time() - this_loop_time.time();
            let time_taken = dur.num_microseconds().unwrap_or(0) as u64;
            if time_taken > crate::config::TICK_TIME_US {
                warn!("Operations performed (time: {}) exceed tick time, overflow...", time_taken);
            }
            /*
                Sleep until next loop tick
            */
            debug!("End loop: {}", this_loop_time);
            sleeper.sleep(std::time::Duration::from_micros(crate::config::TICK_TIME_US));

        }


    });
}



// TODO: leftover, not really used atm 
/*


fn midi_sync(
    &mut self,
    elapsed_time: &Duration,
    current_bpm: i32
) {

    let elapsed_beats = midi_utils::ms_to_beats((elapsed_time).num_milliseconds(), current_bpm);
    self.midi_sync_counter+= elapsed_beats;

    // MIDI Sync allegedly happens 24 times per beat
    let denominator = 1.0 / 24.0;
    if self.midi_sync_counter >= denominator {
        self.osc_client.lock().unwrap().send(
            OscPacket::Message(OscMessage {
                addr: "/midi_sync".to_string(),
                args: vec![]
            })
        );
        self.midi_sync_counter = self.midi_sync_counter - denominator;
    }
}





 */
