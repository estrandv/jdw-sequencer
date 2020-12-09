// Access to playerManager (queue) and sequencerService (bpm)

use rocket_contrib::json::Json;
use rocket::State;
use std::sync::Arc;
use std::sync::Mutex;
use crate::model::rest_input::RestInputNote;
use crate::player_management::PROSCPlayerManager;

#[get("/bpm/<bpm>")]
pub fn set_bpm(bpm: String) {

}

#[post("/queue/prosc/<output_name>", format="json", data="<notes>")]
pub fn queue(
    output_name: String,
    notes: Json<Vec<RestInputNote>>,
    prosc_manager: State<Arc<Mutex<PROSCPlayerManager>>>
) {
    prosc_manager.lock().unwrap().queue(&output_name, notes.into_inner());
}

#[get("/queue/test/<output_name>")]
pub fn test_queue(
    output_name:String,
    prosc_manager: State<Arc<Mutex<PROSCPlayerManager>>>
) {
    prosc_manager.lock().unwrap()
        .queue(&output_name, vec!(
            RestInputNote::new(440, 1.0, 0.5, 1.0),
            RestInputNote::new(640, 1.0, 0.5, 1.0)
        ));
}