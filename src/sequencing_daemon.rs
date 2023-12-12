/*

    Rewrite of... Idunno, main loop? But with the new sequencer/master sequencer classes. 
    Should just tick based on bpm in a thread. 

*/

use std::{sync::{Arc, Mutex}, thread};

use chrono::{DateTime, Utc, Duration};
use log::{info, warn, debug};

use crate::{master_sequencer::MasterSequencer, midi_utils};

struct SequencingDaemonState {
    pub bpm: i32
}

fn start_live_loop <T: 'static + Clone + Send, F> (
    master_sequencer: Arc<Mutex<MasterSequencer<T>>>,
    state: Arc<Mutex<SequencingDaemonState>>,
    entry_operations: F
) where F: 'static + Send + Fn(Vec<T>) -> () {
    thread::spawn(move || {

        let mut last_loop_time: Option<DateTime<Utc>> = None;

        let sleeper = spin_sleep::SpinSleeper::new(100);
        
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
        info!("New master loop began - time taken since last loop (microsec): {:?}", elapsed_time.num_microseconds());


        // TODO: Start, stop, other state injections 

        let current_bpm = state.lock().unwrap().bpm.clone();
        let elapsed_beats = midi_utils::ms_to_beats_bd((elapsed_time).num_milliseconds(), current_bpm);

        /*
            Tick the clock, collect Ts on time. 
        */
        master_sequencer.lock().unwrap().start_check();
        master_sequencer.lock().unwrap().reset_check();
        let collected = master_sequencer.lock().unwrap().tick(elapsed_beats).clone(); // TODO: Does it work without clone, or does that make lock eternal? 
        
        entry_operations(collected);

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


    });
}