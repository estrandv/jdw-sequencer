#![feature(result_flattening, proc_macro_hygiene, decl_macro)]


use std::{cell::RefCell, println, thread};
use std::borrow::Borrow;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use log::{debug, info};
use rosc::{OscBundle, OscMessage, OscPacket};
use simple_logger::SimpleLogger;
use spin_sleep;

use osc_model::{TaggedBundle, TimedOscMessage, UpdateQueueMessage};

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
        // TODO: Proper error handling
        let try_tagged = TaggedBundle::new(&bundle);

        match try_tagged {
            Ok(tagged_bundle) => {
                if &tagged_bundle.bundle_tag == "update_queue" {
                    // TODO: Error handle
                    let update_queue_msg = UpdateQueueMessage::from_bundle(tagged_bundle)
                        .unwrap();

                    let alias = update_queue_msg.alias.clone();

                    log::info!("Updating queue for {}", &alias);
                    self.master_sequencer.lock().unwrap().queue_sequence(
                        &alias, update_queue_msg.messages
                    );

                } else {
                    info!("Unknown tag: {}", &tagged_bundle.bundle_tag)
                }
            }
            Err(e) => info!("Received bundle not parsable as taggedbundle: {}", e),
        }
    }

    fn handle_msg(&self, osc_msg: OscMessage) {
        if osc_msg.addr == "/set_bpm" {
            // TODO: Proper parsing and error handling
            let contained_val = osc_msg.clone().args.get(0).unwrap().clone().int().unwrap();
            self.state_handle.lock().unwrap().bpm.replace(contained_val);
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

    // TODO: Not the cleanest method, especially with the mutable pass...
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
            // TODO: Send a /midi_sync message
            self.midi_sync_counter = self.midi_sync_counter - denominator;
        }
    }


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


            self.midi_sync(&elapsed_time, current_bpm);

            // First send here.
            let messages = self.master_sequence_handler.lock().unwrap().pop_on_time(&this_loop_time);
            for packet in messages {
                self.osc_client.lock().unwrap().send(packet);
            }

            // If there are no notes left to play, reset the sequencer by pushing queues into state
            let all_finished = self.master_sequence_handler.lock().unwrap().all_sequences_finished();
            if all_finished || reset_requested {
                //log::info!("Shifting queues...");
                self.master_sequence_handler.lock().unwrap().shift_queues(current_bpm, &this_loop_time);

                // Second send, since the final tick of a sequence is also the first tick of the next one and
                //  new messages might be available after the shift
                let messages = self.master_sequence_handler.lock().unwrap().pop_on_time(&this_loop_time);
                for packet in messages {
                    self.osc_client.lock().unwrap().send(packet);
                }

            }

            {
                // Force reset means dump everything

                // TODO: Method still relevant?
                if reset_requested || hard_stop_requested {
                    self.master_sequence_handler.lock().unwrap().empty_all();
                }
            }

            let dur = Utc::now().time() - this_loop_time.time();
            let time_taken = dur.num_microseconds().unwrap_or(0) as u64;
            if time_taken > TICK_TIME_US {
                log::warn!("Operations performed (time: {}) exceed tick time, overflow...", time_taken);
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
}
