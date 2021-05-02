#![feature(proc_macro_hygiene, decl_macro)]

use std::{cell::RefCell, println, thread};

use chrono::{DateTime, Utc, Duration};
use model::{OutputTargetType, QueueMetaData, Sequence, SequencerMetaData, SequencerQueueData};
use std::sync::{Arc, Mutex};
use crate::model::SequencerNoteMessage;
use zmq;
use crate::zeromq::PublishingClient;

#[macro_use]
extern crate rocket;

mod model;
pub mod midi_utils;
mod api;
mod external_calls;
mod zeromq;

const TICK_TIME_MS: u64 = 1;
const IDLE_TIME_MS: u64 = 200;

pub struct StateHandle {
    reset: RefCell<bool>,
    hard_stop: RefCell<bool>,
}

fn main() {

    let bpm = Arc::new(Mutex::new(RefCell::new(120)));
    let queue_data: Arc<Mutex<QueueMetaData>> = Arc::new(Mutex::new(QueueMetaData {updated: RefCell::new(false), queue: RefCell::new(Vec::new())}));

    let state_handle: Arc<Mutex<StateHandle>> = Arc::new(Mutex::new(StateHandle{reset: RefCell::new(false), hard_stop: RefCell::new(false)}));

    // Start polling for incoming ZeroMQ messages
    zeromq::poll(
        queue_data.clone(),
        state_handle.clone()
    );

    // Prepare ZeroMQ outgoing client
    let client = Arc::new(Mutex::new(PublishingClient::new()));

    // Get the main loop chugging before initializing the API
    main_loop(bpm.clone(), queue_data.clone(), state_handle.clone(), client);

    rocket::ignite()
        .mount("/", rocket::routes![
            api::set_bpm,
            api::queue_midi,
            api::queue_prosc,
            api::queue_prosc_sample,
            api::reset_queue,
            api::stop
        ])
        .manage(bpm)
        .manage(queue_data)
        .manage(state_handle)
        .launch();
}

fn main_loop(
    bpm: Arc<Mutex<RefCell<i32>>>, // Modified live via API
    queue_data: Arc<Mutex<QueueMetaData>>, // Modified live via API
    state_handle: Arc<Mutex<StateHandle>>, // Modified live via API
    publishing_client: Arc<Mutex<PublishingClient>>,
) {

    thread::spawn(move || {

        let mut state: Vec<SequencerMetaData> = Vec::new();

        let mut last_loop_time: Option<DateTime<Utc>> = Option::None;
        let mut sync_counter: f32 = 0.0;

        loop {

            let this_loop_time = chrono::offset::Utc::now();

            let current_bpm = bpm.lock().unwrap().clone().into_inner();

            {
                let state_handle_lock = state_handle.lock().unwrap();
                // Force reset means dump everything
                if state_handle_lock.reset.clone().into_inner() || state_handle_lock.hard_stop.clone().into_inner() {
                    state = Vec::new();
                }

                if state_handle_lock.hard_stop.clone().into_inner() {
                    queue_data.lock().unwrap().queue.replace(Vec::new());
                }
            }

            let elapsed_beats = match last_loop_time {
                Some(t) => {
                    let dur = this_loop_time.time() - t.time();
                    println!("Tick time (ms): {:?}", dur.num_microseconds().unwrap() as f32 / 1000.0);
                    midi_utils::ms_to_beats((dur).num_milliseconds(), current_bpm)
                },
                None => 0.0
            };

            sync_counter += elapsed_beats;

            // MIDI Sync allegedly happens 24 times per beat 
            if sync_counter > ( 1.0 / 24.0 ) {
                let _res = publishing_client.lock().unwrap().post_midi_sync();
                sync_counter = 0.0;
            }

            last_loop_time = Some(this_loop_time.clone());

            // Play any notes matching the current time
            for meta_data in state.iter_mut() {
                
                let mut on_time = meta_data.active_sequence.get_mut().pop_at_time(this_loop_time.clone());

                // Currently not posting silent notes for performance reasons 
                on_time.retain(|e| e.clone().get_attr("amp").unwrap_or(0.0) > 0.0);

                if !on_time.is_empty() {

                    // The ZMQ posting
                    // TODO: If performance takes a hit, we might need to consider the old way of
                    //  adding all on_time to a collected array and posting them all at once
                    {

                        let post = |note: SequencerNoteMessage| {
                            publishing_client.lock().unwrap().post_note(note);
                        };

                        let post_sample = |note: SequencerNoteMessage| {
                            publishing_client.lock().unwrap().post_sample(note);
                        };

                        let post_midi = |note: SequencerNoteMessage| {
                            publishing_client.lock().unwrap().post_midi_note(note, bpm.lock().unwrap().clone().into_inner());
                        };

                        match meta_data.queue.borrow().target_type {
                            OutputTargetType::Prosc => {
                                on_time.iter().map(|e| e.convert()).for_each(|e| post(e.clone()));
                            },
                            OutputTargetType::ProscSample => {
                                on_time.iter().map(|e| e.convert()).for_each(|e| post_sample(e.clone()));
                            },
                            OutputTargetType::MIDI => {
                                on_time.iter().map(|e| e.convert()).for_each(|e| post_midi(e.clone()));
                            },
                            _ => {}
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
                            let new_data = SequencerMetaData {
                                queue: RefCell::new(queue.clone()),
                                active_sequence: RefCell::new(Sequence::new_empty()),
                            };

                            state.push(new_data);
                        }
                     }
                }

                queue_data.lock().unwrap().updated.replace(false);
            }

            // If there are no notes left to play, reset the sequencer by pushing queues into state
            let all_finished = state.iter().all(|data| data.active_sequence.borrow().is_finished());
            if all_finished || state_handle.lock().unwrap().reset.clone().into_inner() {

                // We cannot rely on the current tick time to supply a new start time, since
                // it might overshoot the final note time by some amount of microseconds.
                // Instead we should find what the latest note time was and start from there.

                let longest_sequence =  state.iter()
                    .max_by_key(|seq| seq.active_sequence.borrow().last_note_time);

                let last_note_time = match longest_sequence {
                    Some(seq) => seq.active_sequence.borrow().last_note_time,
                    None => this_loop_time
                };

                for data in state.iter() {
                    if !data.queue.borrow().queue.borrow().is_empty() {
                        data.active_sequence.replace(Sequence::new(
                            data.queue.borrow().queue.clone().into_inner(),
                            last_note_time,
                            current_bpm.clone())
                        );
                    }
                }
            }

            // Reset will now be handled and can fall back to false
            {
                state_handle.lock().unwrap().reset.replace(false);
                state_handle.lock().unwrap().hard_stop.replace(false);
            }

            if state.is_empty() {
                // Add a little extra wait time when there are no current playing notes
                // to prevent resource waste and allow a window in which to pass multiple initial
                // queues
                println!("Waiting for queue payload...");
                std::thread::sleep(std::time::Duration::from_millis(IDLE_TIME_MS))
            }

            std::thread::sleep(std::time::Duration::from_millis(TICK_TIME_MS));

            //println!("End loop: {}", this_loop_time);            
        } // end loop 
        

    }); // end thread 

}
