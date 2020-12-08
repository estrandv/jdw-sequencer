// Access to playerManager (queue) and sequencerService (bpm)

use rocket_contrib::json::Json;
use rocket::State;
use std::sync::Arc;
use std::sync::Mutex;
use crate::model::rest_input::RestInputNote;

#[get("/bpm/<bpm>")]
pub fn set_bpm(bpm: String) {

}

#[post("/queue/<output_name>", format="json", data="<notes>")]
pub fn queue(
    output_name: String,
    notes: Json<Vec<RestInputNote>>
) {

}

#[get("/queue/test/<output_name>")]
pub fn test_queue(output_name:String) {

}