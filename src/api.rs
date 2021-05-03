use rocket_contrib::json::Json;
use rocket::State;
use std::{cell::RefCell, sync::Arc};
use std::sync::Mutex;

use crate::{StateHandle, model::ApplicationQueue};
use crate::model::{UnprocessedSequence};

#[get("/bpm/<bpm>")]
pub fn set_bpm(
    bpm: i32,
    current_bpm: State<Arc<Mutex<RefCell<i32>>>>
) {
    current_bpm.lock().unwrap().replace(bpm);
}

#[get("/queue/reset")]
pub fn reset_queue(
    state_handle: State<Arc<Mutex<StateHandle>>>
) {
    state_handle.lock().unwrap().reset.replace(true);
}

#[get("/stop")]
pub fn stop(
    state_handle: State<Arc<Mutex<StateHandle>>>
) {
    state_handle.lock().unwrap().hard_stop.replace(true);
}
