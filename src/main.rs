#![feature(result_flattening, proc_macro_hygiene, decl_macro)]


use std::{cell::RefCell, println, thread};

use chrono::{DateTime, Utc, Duration};
use osc_model::{TaggedBundle, UpdateQueueMessage, TimedOscMessage};
use queue::SequencerTickMessage;
use std::sync::{Arc, Mutex};
use rosc::{OscMessage, OscPacket, OscBundle};
use crate::zeromq::PublishingClient;
use spin_sleep;
use crate::osc_client::OSCClient;
use crate::queue::{ApplicationQueue, RealTimeSequence, SequenceHandler};

mod model;
pub mod midi_utils;
mod zeromq;
mod osc_client;
mod queue;
mod osc_model;

// /1000 for ms
const TICK_TIME_US: u64 = 2000;
const IDLE_TIME_MS: u64 = 200;

pub struct StateHandle {
    reset: RefCell<bool>,
    hard_stop: RefCell<bool>,
}

fn main() {

    let bpm = Arc::new(Mutex::new(RefCell::new(120)));
    let queue_data: Arc<Mutex<ApplicationQueue>> = Arc::new(Mutex::new(ApplicationQueue {updated: RefCell::new(false), queue: RefCell::new(Vec::new())}));

    let state_handle: Arc<Mutex<StateHandle>> = Arc::new(Mutex::new(StateHandle{reset: RefCell::new(false), hard_stop: RefCell::new(false)}));

    // Start polling for incoming ZeroMQ messages
    zeromq::poll(
        queue_data.clone(),
        state_handle.clone(),
        bpm.clone()
    );

    // Prepare ZeroMQ outgoing client
    let client = Arc::new(Mutex::new(PublishingClient::new()));

    // Get the main loop chugging before initializing the API

    // TODO: Structure for all this will need some more careful thinking
    let osc_client = OSCClient::new();
    let osc_client_handle = Arc::new(Mutex::new(osc_client));

    main_loop(
        bpm.clone(),
        queue_data.clone(),
        state_handle.clone(),
        client,
    osc_client_handle.clone());


    let osc_read = OSCRead {
        osc_client: osc_client_handle,
        queue_data,
        state_handle,
        bpm
    };

    loop {
        osc_read.scan();
    }

}


// Handle all incoming messages
struct OSCRead {
    osc_client: Arc<Mutex<OSCClient>>,
    queue_data: Arc<Mutex<ApplicationQueue>>,
    state_handle: Arc<Mutex<StateHandle>>,
    bpm: Arc<Mutex<RefCell<i32>>>,
}

impl OSCRead {
    fn scan(&self) {
        match self.osc_client.lock().unwrap().poll() {
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
                println!("{}", error_msg);
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

                    // TODO: Doing it the wonky way first, but long-term we should be able
                    // to pass <alias, vec::timedOsc> straight into update_queue

                    let alias = update_queue_msg.alias.clone();

                    let tick_msgs: Vec<_> = update_queue_msg.messages.iter()
                        .map(|timed_msg: &TimedOscMessage| SequencerTickMessage {
                            alias: alias.clone(),
                            time: timed_msg.time,
                            msg: OscPacket::Message(
                                timed_msg.message.clone()
                            ),
                        })
                        .collect();
                    
                    self.queue_data.lock().unwrap()
                        .update_queue(tick_msgs);
                    
                }
                else {
                    println!("Unknown tag: {}", &tagged_bundle.bundle_tag)
                }
            },
            Err(e) => println!("{}", e),
        }   

    }

    fn handle_msg(&self, osc_msg: OscMessage) {
        if osc_msg.addr == "/update_queue" {
            
        }
        else if osc_msg.addr == "/reset_queue" {
            // Reset by alias
        }
        else if osc_msg.addr == "/set_bpm" {
            // Provide bpm var
        }
        else if osc_msg.addr == "/play" {
            // No contents
        }
        else if osc_msg.addr == "/stop" {
            // No contents
        }
    }
}

// Sequencer ticking
fn main_loop(
    bpm: Arc<Mutex<RefCell<i32>>>, // Modified live via API
    queue_data: Arc<Mutex<ApplicationQueue>>, // Modified live via API
    state_handle: Arc<Mutex<StateHandle>>, // Modified live via API
    publishing_client: Arc<Mutex<PublishingClient>>,
    osc_client: Arc<Mutex<OSCClient>>,
) {

    thread::spawn(move || {

        let mut state: Vec<SequenceHandler> = Vec::new();

        let mut last_loop_time: Option<DateTime<Utc>> = Option::None;
        let mut sync_counter: f32 = 0.0;

        loop {

            let this_loop_time = chrono::offset::Utc::now();
            let elapsed_time = match last_loop_time {
                Some(t) => {
                    this_loop_time.time() - t.time()
                },
                None => Duration::zero()
            };
            last_loop_time = Some(this_loop_time.clone());

            //println!("Loop time (microsec): {:?}", elapsed_time.num_microseconds());

            let current_bpm = bpm.lock().unwrap().clone().into_inner();
            let reset_requested = state_handle.lock().unwrap().reset.clone().into_inner();
            let hard_stop_requested = state_handle.lock().unwrap().hard_stop.clone().into_inner();

            // Since any reset and stop vars are now picked out, we can reset them to false in state
            {
                state_handle.lock().unwrap().reset.replace(false);
                state_handle.lock().unwrap().hard_stop.replace(false);
            }

            let elapsed_beats = midi_utils::ms_to_beats((elapsed_time).num_milliseconds(), current_bpm);
            sync_counter += elapsed_beats;

            // MIDI Sync allegedly happens 24 times per beat
            let denominator = 1.0 / 24.0;
            if sync_counter >= denominator {
                publishing_client.lock().unwrap().post_midi_sync();
                sync_counter = sync_counter - denominator;
            }


            // Play any notes matching the current time
            for meta_data in state.iter_mut() {
                
                let mut on_time = meta_data.active_sequence.get_mut().pop_at_time(this_loop_time.clone());

                if !on_time.is_empty() {

                    //println!("Playing notes at {:?}", chrono::offset::Utc::now());

                    // The ZMQ posting
                    {
                        //let post_time = chrono::offset::Utc::now();
                        let unwrapped: Vec<_> = on_time.iter()
                            .filter(|t| t.message.clone().is_some())
                            .map(|t| t.message.clone().unwrap().msg)
                            .collect();

                        // TODO: Bundle send if necessary
                        for packet in unwrapped {
                            println!("DEBUG: Sending!");
                            osc_client.lock().unwrap().send(packet);
                            println!("DEBUG: Sent!");
                        }
                    }
                }
            }

            // Update the queues if a new queue payload has arrived  
            if queue_data.lock().unwrap().updated.clone().into_inner() || state.is_empty() {
                println!("Updating queue...");
                // Iterate the queues by alias 
                for queue in queue_data.lock().unwrap().queue.clone().into_inner().iter() {
                     
                    let existing = state.iter().find(|data| data.queue.borrow().id == queue.id);

                     // If a queue with the same id exists, we change the queue data according to
                     // request. If not, we create new queue data with an empty sequence to be
                     // populated the next time the queue replaces current. 
                     match existing {
                        Some(old_data) => {
                            old_data.queue.replace(queue.clone());
                        },
                        None => {
                            let new_data = SequenceHandler {
                                queue: RefCell::new(queue.clone()),
                                active_sequence: RefCell::new(RealTimeSequence::new_empty()),
                            };

                            state.push(new_data);
                        }
                     }
                }

                queue_data.lock().unwrap().updated.replace(false);
            }

            // If there are no notes left to play, reset the sequencer by pushing queues into state
            let all_finished = state.iter().all(|data| data.active_sequence.borrow().is_finished());
            if all_finished || reset_requested {

                // We cannot rely on the current tick time to supply a new start time, since
                // it might overshoot the final note time by some amount of microseconds.
                // Instead we should find what the latest note time was and start from there.

                let longest_sequence =  state.iter()
                    .max_by_key(|seq| seq.active_sequence.borrow().last_note_time);

                // Last note time is new start time
                let new_loop_start_time = match longest_sequence {
                    Some(seq) => seq.active_sequence.borrow().last_note_time,
                    None => {
                        // TODO: Spammy before the first queue is created
                        //println!("WARN: No max time found");
                        this_loop_time
                    }
                };

                for data in state.iter() {
                    if !data.queue.borrow().queue.borrow().is_empty() {
                        data.active_sequence.replace(RealTimeSequence::new(
                            data.queue.borrow().queue.clone().into_inner(),
                            new_loop_start_time,
                            current_bpm.clone())
                        );
                    }
                }

                let longest_next =  state.iter()
                    .max_by_key(|seq| seq.active_sequence.borrow().last_note_time);

                let last_next_loop_note_time = match longest_next {
                    Some(seq) => seq.active_sequence.borrow().last_note_time,
                    None => this_loop_time
                };

                println!(
                    "Starting a new loop at time: {}, new loop start time: {}, end time: {}",
                    chrono::offset::Utc::now(),
                    new_loop_start_time,
                    last_next_loop_note_time
                );

                publishing_client.lock().unwrap().post_loop_start(
                    new_loop_start_time,
                    bpm.lock().unwrap().clone().into_inner()
                );

            }

            {
                // Force reset means dump everything
                if reset_requested || hard_stop_requested {
                    state = Vec::new();
                }

                if hard_stop_requested {
                    queue_data.lock().unwrap().queue.replace(Vec::new());
                }
            }

            /*
            if state.is_empty() {
                // Add a little extra wait time when there are no current playing notes
                // to prevent resource waste and allow a window in which to pass multiple initial
                // queues
                println!("Waiting for queue payload...");
                std::thread::sleep(std::time::Duration::from_millis(IDLE_TIME_MS))
            } else {

            }


             */

            let dur = chrono::offset::Utc::now().time() - this_loop_time.time();
            //println!("Full tick (ms): {:?}", dur.num_microseconds().unwrap() as f32 / 1000.0);
            let time_taken = dur.num_microseconds().unwrap_or(0) as u64;
            //println!("Subtracting {} from {}", time_taken, TICK_TIME_US);
            if time_taken > TICK_TIME_US {
                println!("WARN: Operations performed (time: {}) exceed tick time, overflow...", time_taken);
                spin_sleep::sleep(std::time::Duration::from_micros(TICK_TIME_US));
            } else {

                //let remainder = TICK_TIME_US - time_taken;
                let sleeper = spin_sleep::SpinSleeper::new(100);
                sleeper.sleep(std::time::Duration::from_micros(TICK_TIME_US));

            }

            //println!("End loop: {}", this_loop_time);            
        } // end loop 
        

    }); // end thread 

}
