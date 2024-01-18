#![feature(result_flattening, proc_macro_hygiene, decl_macro)]


use std::{cell::RefCell, println, thread};
use std::borrow::Borrow;
use std::process::exit;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use jdw_osc_lib::TaggedBundle;
use log::{debug, info, warn};
use master_sequencer::MasterSequencer;
use rosc::{OscBundle, OscMessage, OscPacket, OscTime, OscType};
use sequencing_daemon::SequencingDaemonState;
use simple_logger::SimpleLogger;
use spin_sleep;
use jdw_osc_polling::{ProcessedOsc, JDWOSCPoller};

use bundle_model::{UpdateQueueMessage};

use crate::config::TICK_TIME_US;
use crate::osc_communication::{OSCClient, OSCPoller};

pub mod midi_utils;
mod osc_communication;
mod bundle_model;
mod config;
mod sequencer;
mod master_sequencer;
mod sequencing_daemon;
mod jdw_osc_polling;


/*

    main.rs starts the two central loops:
        1. The osc polling loop, that handles input and writes it to the shared state. 
        2. The sequencing loop, which reads from input and progresses the sequencers with the passage of time. 

*/


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

    let state_handle: Arc<Mutex<SequencingDaemonState>> = Arc::new(Mutex::new(SequencingDaemonState::new(120)));

    let osc_client = OSCClient::new();
    let osc_client_handle = Arc::new(Mutex::new(osc_client));

    let master = MasterSequencer::new(
        master_sequencer::SequencerStartMode::WithNearestSequence,
         master_sequencer::SequencerResetMode::Individual
    );
    let master_handle = Arc::new(Mutex::new(master));

    // Start sequencer loop in separate thread and handle ticked packets 
    sequencing_daemon::start_live_loop::<OscPacket, _>(master_handle.clone(), state_handle.clone(), move |packets_to_send| {
        
        info!("TICK!");
        
        if !packets_to_send.is_empty() {

            let client_lock = osc_client_handle.lock().unwrap(); 

            for packet in packets_to_send {
                client_lock.send(packet);
            }
        }
    });

    let mut osc_daemon = OSCDaemon {
        poller: JDWOSCPoller::new(),
        state_handle,
        master_sequencer: master_handle
    };

    loop {
        // Take in osc messages
        osc_daemon.scan();
    }
}

// TODO: Can be moved to its own file 
struct OSCDaemon {
    poller: JDWOSCPoller,
    state_handle: Arc<Mutex<SequencingDaemonState>>,
    master_sequencer: Arc<Mutex<MasterSequencer<OscPacket>>>
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

                                        let payload = sequencing_daemon::to_sequence(update_queue_msg.messages);
            
                                        info!("Updating queue for {}", &alias);
                                        self.master_sequencer.lock().unwrap().queue(
                                            &alias, 
                                            payload.message_sequence, 
                                            payload.end_beat
                                        )
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