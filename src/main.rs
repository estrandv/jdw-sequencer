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

pub fn main() {

    let client = RestClient::new();
    let client_ref = Arc::new(Mutex::new(client));
    let prosc_manager = PROSCPlayerManager::new(client_ref.clone());
    let pm_ref = Arc::new(Mutex::new(prosc_manager));
    let daemon = SequencerDaemon::new(pm_ref);

    rocket::ignite()
        .mount("/", routes![api::set_bpm, api::queue, api::test_queue])
        //.manage(midi_ref)
        .launch();
}
