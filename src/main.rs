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
use osc_read_daemon::{ProcessedOsc, JDWOSCPoller};

use osc_model::{UpdateQueueMessage};

use crate::config::TICK_TIME_US;
use crate::osc_client::{OSCClient, OSCPoller};
use crate::queue::SequencerHandler;

pub mod midi_utils;
mod osc_client;
mod osc_model;
mod config;
mod queue;
mod sequencer;
mod master_sequencer;
mod sequencing_daemon;
mod osc_read_daemon;

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

    let osc_client = OSCClient::new();
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

    let mut osc_daemon = OSCDaemon {
        poller: JDWOSCPoller::new(),
        state_handle,
        master_sequencer: master_seq_handle.clone()
    };

    loop {
        // Take in osc messages
        osc_daemon.scan();
    }
}

// TODO: Can be moved to its own file 
struct OSCDaemon {
    poller: JDWOSCPoller,
    state_handle: Arc<Mutex<StateHandle>>,
    master_sequencer: Arc<Mutex<SequencerHandler>>
}

// TODO: Might be better inlined, but sketching is easier like htis 
impl OSCDaemon {
    fn scan(&mut self) {
        match self.poller.scan() {
            Ok(processed_osc) => {
                match processed_osc {
                    ProcessedOsc::Message(osc_msg) => {
                        match osc_msg.addr.as_str() {
                            "/set_bpm" => {
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
                            },
                            "/play" => {
                                // No contents 
                            },
                            "/reset_all" => {
                                self.state_handle.lock().unwrap().reset.replace(true);
                            },
                            "/hard_stop" => {
                                self.state_handle.lock().unwrap().hard_stop.replace(true);
                            },
                            _ => {}
                
                        }
                    },
                    ProcessedOsc::Bundle(tagged_bundle) => {
                        match tagged_bundle.bundle_tag.as_str() {
                            "update_queue" => {
                                match UpdateQueueMessage::from_bundle(tagged_bundle) {
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
                            },
                            _ => {
                                info!("Unknown tag: {}", &tagged_bundle.bundle_tag)
                            }
                        } 
                    }
                }
            },
            Err(msg) => {
                info!("Error processing incoming osc: {}", msg);
            } 
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

            /*
                Consume user input variables from state and reset them where needed.
            */
            let current_bpm = self.state_handle.lock().unwrap().bpm.clone().into_inner();
            let reset_requested = self.state_handle.lock().unwrap().reset.clone().into_inner();
            let hard_stop_requested = self.state_handle.lock().unwrap().hard_stop.clone().into_inner();
            {
                self.state_handle.lock().unwrap().reset.replace(false);
                self.state_handle.lock().unwrap().hard_stop.replace(false);
            }

            /*
                Send a midi sync message every loop, if enabled.
                TODO: Not really used or tested. Needs to be a packet-getter method instead of locking the client. 
            */
            if config::MIDI_SYNC {
                self.midi_sync(&elapsed_time, current_bpm);
            }

            /*
                Determine how many "beats" have passed since the last loop (from time and current bpm).
                Use this to collect beat-triggered sequencer packets.
            */

            // Begin collecting loop packets to send all at once - some come from ticks, some from queue shifting, etc. 
            let mut packets_to_send: Vec<OscPacket> = Vec::new();

            let elapsed_beats = midi_utils::ms_to_beats((elapsed_time).num_milliseconds(), current_bpm);
            let messages = self.master_sequence_handler.lock().unwrap().tick_and_return_all(&elapsed_beats);
            for packet in messages {
                packets_to_send.push(packet);
            }

            /*
                Check if any new sequencers have been created since last loop, starting them if
                    start criteria are met (typically on some kind of sequence start unless configured as "immediate start")
            */            
            self.master_sequence_handler.lock().unwrap().start_all_new();

            /*
                QUEUE RESET OPTION 1: Handle manual and "everything is done and waiting" queue resets.
            */
            let all_finished = self.master_sequence_handler.lock().unwrap().all_sequences_finished();
            if all_finished || reset_requested {

                info!("All queues finished, shifting queues...");

                // On shift, we start immediately on the new timeline (getting any 0.0 packets as oversend)
                let oversend = self.master_sequence_handler.lock().unwrap().shift_queues();

                for packet in oversend {
                    packets_to_send.push(packet);
                }

                /*
                    A total reset also triggers a special "full loop started" message. 
                    TODO: Not really tested or used - should probably trigger in other scenarios as well.
                */
                packets_to_send.push(
                    OscPacket::Message(OscMessage {
                        addr: "/jdw_seq_loop_start".to_string(),
                        args: vec![
                            // TODO: Start time is the internal one from "shift queues", should prob be sent
                            // in some common date-string format
                        ]
                    })
                );

            /*
                QUEUE RESET OPTION 2: Handle queue resets for individual reset mode 
            */
            } else if config::SEQUENER_RESET_MODE == config::SEQ_RESET_MODE_INDIVIDUAL {

                let oversend = self.master_sequence_handler.lock().unwrap().shift_finished();

                // TODO: What is a "loop start" message in this case? individual?

                for packet in oversend {
                    packets_to_send.push(packet);
                }
            }

            /*
                Handle manual resets and hard stops by effectively removing all running sequences. 
            */
            {
                // Force reset means dump everything

                if reset_requested || hard_stop_requested {
                    self.master_sequence_handler.lock().unwrap().empty_all();
                }
            }


            /*
                Send all packets collected for this loop tick
            */
            {

                if !packets_to_send.is_empty() {

                    let client_lock = self.osc_client.lock().unwrap(); 

                    for packet in packets_to_send {
                        client_lock.send(packet);
                    }
                }

            }


            /*
                Calculate time taken to execute this loop and log accordingly
            */
            let dur = Utc::now().time() - this_loop_time.time();
            let time_taken = dur.num_microseconds().unwrap_or(0) as u64;
            if time_taken > TICK_TIME_US {
                warn!("Operations performed (time: {}) exceed tick time, overflow...", time_taken);
            }
            // NOTE: Previously, if time taken did not overshoot tick time, we subtracted it to make all ticks have roughly the same effective time
            //  I've never found any evidence of this having any effect on the "feel" of the sequencer, but here's how it went:
            // let remainder = TICK_TIME_US - time_taken;


            /*
                Sleep until next loop tick
            */
            debug!("End loop: {}", this_loop_time);
            sleeper.sleep(std::time::Duration::from_micros(TICK_TIME_US));

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
