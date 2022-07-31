#![feature(result_flattening, proc_macro_hygiene, decl_macro)]


use std::{cell::RefCell, println, thread};
use std::borrow::Borrow;
use std::process::exit;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use jdw_osc_lib::TaggedBundle;
use log::{debug, info, warn};
use rosc::{OscBundle, OscMessage, OscPacket, OscTime, OscType};
use simple_logger::SimpleLogger;
use spin_sleep;

use osc_model::{UpdateQueueMessage};

use crate::config::TICK_TIME_US;
use crate::osc_client::{OSCClient, OSCPoller};
use crate::queue::SequencerHandler;

pub mod midi_utils;
mod osc_client;
mod osc_model;
mod config;
mod queue;

// /1000 for ms
//const IDLE_TIME_MS: u64 = 200;

pub struct StateHandle {
    reset: RefCell<bool>,
    hard_stop: RefCell<bool>,
    bpm: RefCell<i32>,
}

fn main() {

    // Handles all log macros, e.g. "warn!()" to print info in terminal
    SimpleLogger::new()
        .with_level(config::LOG_LEVEL)
        .init().unwrap();

    let state_handle: Arc<Mutex<StateHandle>> = Arc::new(Mutex::new(StateHandle {
        reset: RefCell::new(false), hard_stop: RefCell::new(false), bpm: RefCell::new(120)
    }));

    let osc_poller = OSCPoller::new();
    let osc_client = OSCClient::new();
    let osc_poller_handle = Arc::new(Mutex::new(osc_poller));
    let osc_client_handle = Arc::new(Mutex::new(osc_client));

    let master_sequencer = SequencerHandler::new();
    let master_seq_handle = Arc::new(Mutex::new(master_sequencer));

    // Start sequencer loop in separate thread
    let mut main_loop = SequencerTickLoop {
        state_handle: state_handle.clone(),
        osc_client: osc_client_handle.clone(),
        master_sequence_handler: master_seq_handle.clone(),
        midi_sync_counter: 0.0
    };


    thread::spawn(move || {
        main_loop.run();
    }); // end thread

    let osc_read = OSCRead {
        poller: osc_poller_handle,
        state_handle,
        master_sequencer: master_seq_handle.clone()
    };

    loop {
        // Take in osc messages
        osc_read.scan();
    }
}


// Handle all incoming messages
struct OSCRead {
    poller: Arc<Mutex<OSCPoller>>,
    state_handle: Arc<Mutex<StateHandle>>,
    master_sequencer: Arc<Mutex<SequencerHandler>>
}

impl OSCRead {
    fn scan(&self) {
        match self.poller.lock().unwrap().poll() {
            Ok(osc_packet) => {
                match osc_packet {
                    OscPacket::Message(osc_msg) => {
                        self.handle_msg(osc_msg);
                    }
                    OscPacket::Bundle(osc_bundle) => {
                        self.handle_bundle(osc_bundle);
                    }
                };
            }
            Err(error_msg) => {
                log::warn!("{}", error_msg);
            }
        }
    }

    fn handle_bundle(&self, bundle: OscBundle) {
        let try_tagged = TaggedBundle::new(&bundle);

        info!("Tagged bundle received! {:?}", &bundle);
        
        match try_tagged {
            Ok(tagged_bundle) => {


                if &tagged_bundle.bundle_tag == "update_queue" {
                    let update_queue_msg_res = UpdateQueueMessage::from_bundle(tagged_bundle);

                    match update_queue_msg_res {
                        Ok(update_queue_msg) => {

                            let alias = update_queue_msg.alias.clone();

                            info!("Updating queue for {}", &alias);
                            self.master_sequencer.lock().unwrap().queue_sequence(
                                &alias, update_queue_msg.messages
                            );
                        }
                        Err(e) => {
                            warn!("Failed to parse update_queue message: {}", e);
                        }
                    }

                } else {
                    info!("Unknown tag: {}", &tagged_bundle.bundle_tag)
                }
            }
            Err(e) => info!("Received bundle not parsable as taggedbundle: {}", e),
        }
    }

    fn handle_msg(&self, osc_msg: OscMessage) {
        if osc_msg.addr == "/set_bpm" {
            let args = osc_msg.clone().args;
            let arg = args.get(0).clone();
            match arg {
                None => {
                    warn!("Unable to parse set_bpm message (missing arg)")
                }
                Some(val) => {
                    match val.clone().int() {
                        None => {warn!("set_bpm arg not an int")}
                        Some(contained_val) => {
                            self.state_handle.lock().unwrap().bpm.replace(contained_val);

                        }
                    }
                }
            }
        } else if osc_msg.addr == "/play" {
            // No contents
        } else if osc_msg.addr == "/reset_all" {
            self.state_handle.lock().unwrap().reset.replace(true);
        } else if osc_msg.addr == "/hard_stop" {
            self.state_handle.lock().unwrap().hard_stop.replace(true);
        }
    }
}

struct SequencerTickLoop {
    state_handle: Arc<Mutex<StateHandle>>, // Modified live via API
    osc_client: Arc<Mutex<OSCClient>>,
    master_sequence_handler: Arc<Mutex<SequencerHandler>>,
    midi_sync_counter: f32
}

impl SequencerTickLoop {

    fn run(&mut self) {

        let mut last_loop_time: Option<DateTime<Utc>> = None;

        let sleeper = spin_sleep::SpinSleeper::new(100);

        loop {
            let this_loop_time = Utc::now();
            let elapsed_time = match last_loop_time {
                Some(t) => {
                    this_loop_time.time() - t.time()
                }
                None => Duration::zero()
            };
            last_loop_time = Some(this_loop_time.clone());

            debug!("Loop time (microsec): {:?}", elapsed_time.num_microseconds());

            let current_bpm = self.state_handle.lock().unwrap().bpm.clone().into_inner();
            let reset_requested = self.state_handle.lock().unwrap().reset.clone().into_inner();
            let hard_stop_requested = self.state_handle.lock().unwrap().hard_stop.clone().into_inner();

            // Since any reset and stop vars are now picked out, we can reset them to false in state
            {
                self.state_handle.lock().unwrap().reset.replace(false);
                self.state_handle.lock().unwrap().hard_stop.replace(false);
            }

            // TODO: Inconvenient spam when not using router
            //self.midi_sync(&elapsed_time, current_bpm);

            let elapsed_beats = midi_utils::ms_to_beats((elapsed_time).num_milliseconds(), current_bpm);

            // First send here.
            let messages = self.master_sequence_handler.lock().unwrap().tick_and_return_all(&elapsed_beats);
            for packet in messages {
                self.osc_client.lock().unwrap().send(packet);
            }

            // TODO: This should then be a config
            let mode_individual = true;

            // If there are no notes left to play, reset the sequencer by pushing queues into state
            let all_finished = self.master_sequence_handler.lock().unwrap().all_sequences_finished();
            if all_finished || reset_requested {

                // On shift, we start immediately on the new timeline (getting any 0.0 packets as oversend)
                let oversend = self.master_sequence_handler.lock().unwrap().shift_queues();

                self.osc_client.lock().unwrap().send(
                    OscPacket::Message(OscMessage {
                        addr: "/jdw_seq_loop_start".to_string(),
                        args: vec![
                            // TODO: Start time is the internal one from "shift queues", should prob be sent
                            // in some common date-string format
                        ]
                    })
                );

                // Second send, since the final tick of a sequence is also the first tick of the next one and
                //  new messages might be available after the shift

                // TODO: Single-send idea.
                // - Return overshoot notes when shift happens
                // - Must not even necessarily be an overshoot - just return the 0.0 ones
                // - Still have to send again... but in a less repetitive way
                // - Overshoot is tricky as usual: Some time HAS passed since the elapsed beats calculation
                //  - Time until last tick is completely negligible however. The problem is if it shifts the total somehow.
                /*
                    EXPERIMENT: SHift times

                    loop 2 start time: 10
                    tick: Every 3 seconds
                    end time: 10
                    last tick is at: 12 seconds
                    So even as the tick starts we have an overshoot of 2 seconds.
                    We immediately send the last note by ticking.
                    We shift queues. The timeline in the sequencer overshoots and is now 2.
                    Time is now closer to "13" however since the shifting has taken some time.
                    We immediately send the remaining notes.

                    New loop time is now 2, but next tick will add 3 or 4 or whatever time has passed since last tick.
                    The time after shifting is 12 + shift_time
                    In the sequence, the time is 2 (12)

                    Next tick comes on.
                    Time is now 12 + shift_time + 3 = 15s
                    Sequence takes in this formula, so that time is 5s
                    And so we're back on track!

                    Only issue is anything that might happen after 0.0 (by s) but before whatever tiny fucking increment
                    happens before the next tick.

                 */
                for packet in oversend {
                    self.osc_client.lock().unwrap().send(packet);
                }

            } else if mode_individual {

                // TODO: Messy code
                let oversend = self.master_sequence_handler.lock().unwrap().shift_finished();

                // TODO: What is a "loop start" message in this case? individual? 

                for packet in oversend {
                    self.osc_client.lock().unwrap().send(packet);
                }
            }

            {
                // Force reset means dump everything

                if reset_requested || hard_stop_requested {
                    self.master_sequence_handler.lock().unwrap().empty_all();
                }
            }

            let dur = Utc::now().time() - this_loop_time.time();
            let time_taken = dur.num_microseconds().unwrap_or(0) as u64;
            if time_taken > TICK_TIME_US {
                warn!("Operations performed (time: {}) exceed tick time, overflow...", time_taken);
                spin_sleep::sleep(std::time::Duration::from_micros(TICK_TIME_US));
            } else {

                // NOTE: Evening out the tick like this is not required,
                //  since even tick time is not theoretically required to
                //  catch any message by-time sufficiently for the human ear.
                // Figured it was a nice-to-have anyway...
                let remainder = TICK_TIME_US - time_taken;
                sleeper.sleep(std::time::Duration::from_micros(remainder));
            }

            debug!("End loop: {}", this_loop_time);
        } // end loop
    }

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

}
