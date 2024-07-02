#![feature(result_flattening, proc_macro_hygiene, decl_macro)]


use std::cell::RefCell;


use std::sync::{Arc, Mutex};

use chrono::Utc;
use log::{info, warn};
use master_sequencer::MasterSequencer;
use rosc::OscPacket;
use sequencing_daemon::SequencingDaemonState;
use simple_logger::SimpleLogger;

use bundle_model::{UpdateQueueMessage};

use crate::osc_communication::OSCClient;
use jdw_osc_lib::osc_stack::OSCStack;
use crate::bundle_model::BatchUpdateQueuesMessage;

pub mod midi_utils;
mod osc_communication;
mod bundle_model;
mod config;
mod sequencer;
mod master_sequencer;
mod sequencing_daemon;


/*

    main.rs starts the two central loops:
        1. The osc polling loop, that handles input and writes it to the shared state. 
        2. The sequencing loop, which reads from input and progresses the sequencers with the passage of time. 

*/

fn main() {

    // Handles all log macros, e.g. "warn!()" to print info in terminal
    SimpleLogger::new()
        .with_level(config::LOG_LEVEL)
        .init().unwrap();

    let state_handle: Arc<Mutex<SequencingDaemonState>> = Arc::new(Mutex::new(SequencingDaemonState::new(120)));

    let osc_client = OSCClient::new();
    let osc_client_handle = Arc::new(Mutex::new(osc_client));

    let master = MasterSequencer::new(
        master_sequencer::SequencerStartMode::WithLongestSequence,
         master_sequencer::SequencerResetMode::Individual
    );
    let master_handle = Arc::new(Mutex::new(master));

    // Start sequencer loop in separate thread and handle ticked packets 
    sequencing_daemon::start_live_loop::<OscPacket, _>(master_handle.clone(), state_handle.clone(), move |packets_to_send| {
        
        
        if !packets_to_send.is_empty() {

            info!("TICK! {:?}", Utc::now());

            let client_lock = osc_client_handle.lock().unwrap(); 

            for packet in packets_to_send {
                client_lock.send(packet);
            }
        }
    });

    let addr = config::get_addr(config::APPLICATION_IN_PORT);
    OSCStack::init(addr)
        .on_message("/set_bpm", &|msg| {
            let args = msg.clone().args;
            let arg = args.get(0).clone();
            match arg {
                None => {
                    warn!("Unable to parse set_bpm message (missing arg)")
                }
                Some(val) => {
                    match val.clone().int() {
                        None => {warn!("set_bpm arg not an int")}
                        Some(contained_val) => {
                            state_handle.lock().unwrap().bpm.replace(contained_val);
                        }
                    }
                }
            }    
        })
        .on_message("/reset_all", &|_msg| {
            state_handle.lock().unwrap().reset.replace(true);
        })
        .on_message("/hard_stop", &|_msg| {
            state_handle.lock().unwrap().hard_stop.replace(true);
        })
        .on_message("/wipe_on_finish", &|_msg| {
            master_handle.lock().unwrap().end_after_finish();
        })
        .on_tbundle("batch_update_queues", &|tbundle| {

            match BatchUpdateQueuesMessage::from_bundle(tbundle) {
                Ok(batch_update_msg) => {

                    if batch_update_msg.stop_missing {
                        // Same as a call to "wipe on finish" - the order is then immediately reversed
                        //  for mentioned tracks when a new queue() is called
                        master_handle.lock().unwrap().end_after_finish();
                    }

                    for update_queue_msg in batch_update_msg.update_queue_messages {
                        let alias = update_queue_msg.alias.clone();

                        let payload = sequencing_daemon::to_sequence(update_queue_msg.messages);

                        info!("Updating queue for {}", &alias);

                        master_handle.lock().unwrap().queue(
                            &alias,
                            payload.message_sequence,
                            payload.end_beat,
                            update_queue_msg.one_shot
                        );
                    }
                }
                Err(e) => {
                    warn!("Failed to parse batch update queue message: {}", e);
                }
            }


        })
        .on_tbundle("update_queue", &|tbundle| {
            match UpdateQueueMessage::from_bundle(tbundle) {
                Ok(update_queue_msg) => {

                    let alias = update_queue_msg.alias.clone();

                    let payload = sequencing_daemon::to_sequence(update_queue_msg.messages);

                    info!("Updating queue for {}", &alias);

                    master_handle.lock().unwrap().queue(
                        &alias, 
                        payload.message_sequence, 
                        payload.end_beat,
                        update_queue_msg.one_shot
                    );
                }
                Err(e) => {
                    warn!("Failed to parse update_queue message: {}", e);
                }
            }
        })
        .begin();
}