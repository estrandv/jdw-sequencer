use crate::rest::RestClient;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::sequence_player::SequencePlayer;
use chrono::{DateTime, Utc};
use crate::model::rest_input::RestInputNote;

pub struct PROSCPlayerManager {
    prosc_players: Arc<Mutex<HashMap<String, Arc<Mutex<SequencePlayer>>>>>,
    rest_client: Arc<Mutex<RestClient>>
}

impl PROSCPlayerManager {

    pub fn new(rc: Arc<Mutex<RestClient>>) -> PROSCPlayerManager {
        PROSCPlayerManager {
            prosc_players: Arc::new(Mutex::new(HashMap::new())),
            rest_client: rc
        }
    }

    pub fn play_next(&self, time: DateTime<Utc>, bpm: i32) {

        for (name, player) in self.prosc_players.lock().unwrap().iter() {
            let notes_on_time = player.lock().unwrap().get_next(time, bpm);
            if notes_on_time.len() > 1 {
                println!("WARNING: Note overflow!");
                self.rest_client.lock().unwrap().post_prosc(name, notes_on_time);
            }
        }
    }

    pub fn queue(&self, output_name: &str, notes: Vec<RestInputNote>) {
        if !self.prosc_players.lock().unwrap().contains_key(output_name) {
            self.prosc_players.lock().unwrap().insert(
                output_name.to_string(),
                Arc::new(Mutex::new(SequencePlayer::new()))
            );
        }

        self.prosc_players.lock().unwrap().get(output_name).unwrap().lock().unwrap()
            .queue(notes);
    }

}