// Access to playerManager (queue) and sequencerService (bpm)

use rocket_contrib::json::Json;
use rocket::State;
use std::sync::Arc;
use std::sync::Mutex;
use crate::model::rest_input::RestInputNote;
use crate::player_management::PROSCPlayerManager;
use crate::daemon::SequencerDaemon;

#[get("/bpm/<bpm>")]
pub fn set_bpm(
    daemon: State<Arc<Mutex<SequencerDaemon>>>,
    bpm: i32
) {
    daemon.lock().unwrap().bpm(bpm);
}
/*
    Queue notes to registered prosc output of name
    Use the given alias; with differnet aliases you can queue several sets to the same
        output. Queueing to the same alias will replace the notes/output for that alias on its next loop.
 */
#[post("/queue/prosc/<output_name>/<alias>", format="json", data="<notes>")]
pub fn queue(
    output_name: String,
    alias: String,
    notes: Json<Vec<RestInputNote>>,
    prosc_manager: State<Arc<Mutex<PROSCPlayerManager>>>
) {
    prosc_manager.lock().unwrap().queue_prosc(&output_name, &alias, notes.into_inner());
}

#[get("/queue/test/<output_name>")]
pub fn test_queue(
    output_name:String,
    prosc_manager: State<Arc<Mutex<PROSCPlayerManager>>>
) {
    prosc_manager.lock().unwrap()
        .queue_prosc(&output_name, "testQueue", vec!(
            RestInputNote::new(440.0, 1.0, 0.5, 1.0),
            RestInputNote::new(640.0, 2.0, 1.0, 1.0),
        ));
}