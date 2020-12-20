#![feature(proc_macro_hygiene, decl_macro)]
#![feature(cell_update)]
#[macro_use] extern crate rocket;

mod api;
mod model;
pub mod daemon;
pub mod player_management;
pub mod rest;
pub mod sequence_player;

use crate::api::*;
use crate::rest::RestClient;
use crate::player_management::PROSCPlayerManager;
use std::sync::{Arc, Mutex};
use crate::daemon::SequencerDaemon;
use crate::model::sequencer::SequencerNote;

pub fn main() {

    let client = RestClient::new();
    let client_ref = Arc::new(Mutex::new(client));
    let prosc_manager = PROSCPlayerManager::new(client_ref.clone());
    let pm_ref = Arc::new(Mutex::new(prosc_manager));
    let daemon = SequencerDaemon::new(pm_ref.clone());
    let daemon_ref = Arc::new(Mutex::new(daemon));

    {
        SequencerDaemon::start(daemon_ref.clone());
    }

    // Debugging, but also in a sense a wakeup tone!
    {
        client_ref.clone().lock().unwrap().local_post_prosc("blipp", vec!(SequencerNote {
            tone: 440.0,
            amplitude: 1.0,
            sustain: 1.0,
            start_beat: 0.0
        }));
    }

    rocket::ignite()
        .mount("/", routes![api::set_bpm, api::queue, api::test_queue])
        .manage(pm_ref)
        .manage(daemon_ref)
        .launch();
}
