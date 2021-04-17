#![feature(proc_macro_hygiene, decl_macro)]

use std::{cell::RefCell, println, thread};

use chrono::{DateTime, Utc};
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

            let current_bpm = bpm.lock().unwrap().clone().into_inner();

            let this_loop_time = chrono::offset::Utc::now();

            let elapsed_beats = match last_loop_time {
                Some(t) => { midi_utils::ms_to_beats((this_loop_time.time() - t.time()).num_milliseconds(), current_bpm)},
                None => 0.0
            };

            sync_counter += elapsed_beats;

            // MIDI Sync allegedly happens 24 times per beat 
            if sync_counter > ( 1.0 / 24.0 ) {
                let _res = external_calls::sync_midi();
                sync_counter = 0.0;
            }

            last_loop_time = Some(this_loop_time.clone());

            let mut collected_synth: Vec<SequencerNoteMessage> = Vec::new();
            let mut collected_sample: Vec<SequencerNoteMessage> = Vec::new();

            for meta_data in state.iter_mut() {
                
                let mut on_time = meta_data.active_sequence.get_mut().pop_at_time(this_loop_time.clone());

                // Currently not posting silent notes for performance reasons 
                on_time.retain(|e| e.clone().get_attr("amp").unwrap_or(0.0) > 0.0);

                if !on_time.is_empty() {

                    let instrument_id = meta_data.queue.borrow().instrument_id.clone();

                    // The ZMQ posting
                    // TODO: If performance takes a hit, we might need to consider the old way of
                    //  adding all on_time to a collected array and posting them all at once
                    {

                        let post = |note: SequencerNoteMessage| {
                            publishing_client.lock().unwrap().post_note(note);
                        };

                        on_time.iter().map(|e| e.convert()).for_each(|e| post(e.clone()));
                    }

                    // The REST posting, later to be removed
                    println!("id:{} -> {}", meta_data.queue.borrow().id, &instrument_id);
                    match meta_data.queue.borrow().target_type {
                        OutputTargetType::Prosc => {
                            on_time.iter().map(|e| e.convert()).for_each(|e| collected_synth.push(e.clone()));
                        },
                        OutputTargetType::MIDI => {
                            let _result = external_calls::post_midi_notes(&instrument_id, on_time);
                        },
                        OutputTargetType::ProscSample => {
                            on_time.iter().map(|e| e.convert()).for_each(|e| collected_sample.push(e.clone()));
                        },
                    }
                }

            }

            if !collected_sample.is_empty() {
                let _res = external_calls::post_prosc_samples(collected_sample);
            }

            if !collected_synth.is_empty() {
                let _res = external_calls::post_prosc_notes(collected_synth);
            }

            // Only push queue into current state if needed  
            if queue_data.lock().unwrap().updated.clone().into_inner() || state.is_empty() {
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

            // Replace all now empty active sequences with their queue counterparts (resetting)
            let all_finished = state.iter().all(|data| data.active_sequence.borrow().is_finished());
            if all_finished || state_handle.lock().unwrap().reset.clone().into_inner() {
                for data in state.iter() {
                    if !data.queue.borrow().queue.borrow().is_empty() {
                        data.active_sequence.replace(Sequence::new(data.queue.borrow().queue.clone().into_inner(), this_loop_time, current_bpm.clone()));
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
                std::thread::sleep(std::time::Duration::from_millis(IDLE_TIME_MS))
            }

            std::thread::sleep(std::time::Duration::from_millis(TICK_TIME_MS));

            //println!("End loop: {}", this_loop_time);            
        } // end loop 
        

    }); // end thread 

}
