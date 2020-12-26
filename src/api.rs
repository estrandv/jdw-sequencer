// Access to playerManager (queue) and sequencerService (bpm)

use rocket_contrib::json::Json;
use rocket::State;
use std::sync::Arc;
use std::sync::Mutex;
use crate::model::rest_input::RestInputNote;
use crate::player_management::PlayerManager;
use crate::daemon::SequencerDaemon;
use crate::sequence_player::PlayerTarget;

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
pub fn queue_prosc(
    output_name: String,
    alias: String,
    notes: Json<Vec<RestInputNote>>,
    player_manager: State<Arc<Mutex<PlayerManager>>>
) {
    player_manager.lock().unwrap().queue_notes(PlayerTarget::PROSC, &output_name, &alias, notes.into_inner());
}

#[post("/queue/prosc_sample/<output_name>/<alias>", format="json", data="<notes>")]
pub fn queue_prosc_sample(
    output_name: String,
    alias: String,
    notes: Json<Vec<RestInputNote>>,
    player_manager: State<Arc<Mutex<PlayerManager>>>
) {
    player_manager.lock().unwrap().queue_notes(PlayerTarget::PROSC_SAMPLE, &output_name, &alias, notes.into_inner());
}

#[post("/queue/midi/<output_name>/<alias>", format="json", data="<notes>")]
pub fn queue_midi(
    output_name: String,
    alias: String,
    notes: Json<Vec<RestInputNote>>,
    player_manager: State<Arc<Mutex<PlayerManager>>>
) {
    player_manager.lock().unwrap().queue_notes(PlayerTarget::MIDI, &output_name, &alias, notes.into_inner());
}

#[get("/queue/test/<output_name>")]
pub fn test_queue(
    output_name:String,
    prosc_manager: State<Arc<Mutex<PlayerManager>>>
) {
    prosc_manager.lock().unwrap()
        .queue_notes(PlayerTarget::PROSC, &output_name, "testQueue", vec!(
            RestInputNote::new(440.0, 1.0, 0.5, 1.0),
            RestInputNote::new(640.0, 2.0, 1.0, 1.0),
        ));
}