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

pub fn main() {

    rocket::ignite()
        .mount("/", routes![api::set_bpm, api::queue, api::testQueue])
        //.manage(midi_ref)
        .launch();
}
