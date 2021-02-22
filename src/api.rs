// Access to playerManager (queue) and sequencerService (bpm)

use rocket_contrib::json::Json;
use rocket::State;
use std::{cell::RefCell, sync::Arc};
use std::sync::Mutex;

use crate::external_calls::SNewMessage;
use crate::model::{RestInputNote, SequencerQueueData};

#[get("/bpm/<bpm>")]
pub fn set_bpm(
    bpm: i32,
    current_bpm: State<Arc<Mutex<RefCell<i32>>>>
) {
    current_bpm.lock().unwrap().replace(bpm);
}

#[get("/queue/reset")]
pub fn reset_queue(
    reset_handle: State<Arc<Mutex<RefCell<bool>>>>
) {
    reset_handle.lock().unwrap().replace(true);
}

/*
    Queue notes to registered prosc output of name
    Use the given alias; with differnet aliases you can queue several sets to the same
        output. Queueing to the same alias will replace the notes/output for that alias on its next loop.
 */
#[post("/queue/prosc/<output_name>/<alias>", format="json", data="<notes>")]
pub fn queue_prosc(
    output_name: String,
    alias: String,
    notes: Json<Vec<SNewMessage>>,
    queue_data: State<Arc<Mutex<RefCell<Vec<SequencerQueueData>>>>>,
) {

    queue_data.lock().unwrap().borrow_mut().retain(|e| *e.id != alias);
 
    queue_data.lock().unwrap().borrow_mut().push(SequencerQueueData {
        id: alias,
        target_type: crate::model::OutputTargetType::Prosc,
        instrument_id: output_name,
        queue: RefCell::new(notes.into_inner())
    }); 

}

#[post("/queue/prosc_sample/<output_name>/<alias>", format="json", data="<notes>")]
pub fn queue_prosc_sample(
    output_name: String,
    alias: String,
    notes: Json<Vec<SNewMessage>>,
    queue_data: State<Arc<Mutex<RefCell<Vec<SequencerQueueData>>>>>,
) {

    queue_data.lock().unwrap().borrow_mut().retain(|e| *e.id != alias);
 
    queue_data.lock().unwrap().borrow_mut().push(SequencerQueueData {
        id: alias,
        target_type: crate::model::OutputTargetType::ProscSample,
        instrument_id: output_name,
        queue: RefCell::new(notes.into_inner())
    }); 

}

#[post("/queue/midi/<output_name>/<alias>", format="json", data="<notes>")]
pub fn queue_midi(
    output_name: String,
    alias: String,
    notes: Json<Vec<SNewMessage>>,
    queue_data: State<Arc<Mutex<RefCell<Vec<SequencerQueueData>>>>>,
) {
 
    queue_data.lock().unwrap().borrow_mut().retain(|e| *e.id != alias);
 
    queue_data.lock().unwrap().borrow_mut().push(SequencerQueueData {
        id: alias,
        target_type: crate::model::OutputTargetType::MIDI,
        instrument_id: output_name,
        queue: RefCell::new(notes.into_inner())
    });    
}
