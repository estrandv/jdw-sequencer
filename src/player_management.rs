use crate::rest::RestClient;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::sequence_player::SequencePlayer;
use chrono::{DateTime, Utc};
use crate::model::rest_input::RestInputNote;
use std::cell::{Cell, RefCell};

/*
    Organizer for sequence players targeting PROSC.
 */
pub struct PROSCPlayerManager {
    prosc_players: Arc<Mutex<HashMap<String, Arc<Mutex<SequencePlayer>>>>>,
    rest_client: Arc<Mutex<RestClient>>,
    longest_active_player_dur: Arc<Mutex<Cell<f32>>>
}

impl PROSCPlayerManager {

    pub fn new(rc: Arc<Mutex<RestClient>>) -> PROSCPlayerManager {
        PROSCPlayerManager {
            prosc_players: Arc::new(Mutex::new(HashMap::new())),
            rest_client: rc,
            longest_active_player_dur: Arc::new(Mutex::new(Cell::new(0.0))),
        }
    }

    // Scan for upcoming notes in all players and send to PROSC where appropriate
    pub fn play_next(&self, time: DateTime<Utc>, bpm: i32) {

        let all_finished = self.prosc_players.lock().unwrap().values().into_iter()
            .all(|player| player.lock().unwrap().is_finished());

        if all_finished {
            for (_, player) in self.prosc_players.lock().unwrap().iter() {
                player.lock().unwrap().shift_queue(time);
            }
        }

        for (_, player) in self.prosc_players.lock().unwrap().iter() {
            let notes_on_time = player.lock().unwrap().get_next(time, bpm);
            if notes_on_time.len() > 1 {
                println!("WARNING: Note overflow!");
            }
            if !notes_on_time.is_empty() {

                let url = player.lock().unwrap().target_url.clone();
                let output = player.lock().unwrap().target_output.lock().unwrap().clone();

                self.rest_client.clone().lock().unwrap()
                    .post_prosc_notes(&url, &output, notes_on_time.clone());
            }
        }
    }

    // Queue a set of notes for the given output name.
    // Non-existing output players will be created.
    pub fn queue_prosc(&self, output_name: &str, alias: &str, notes: Vec<RestInputNote>) {
        if !self.prosc_players.lock().unwrap().contains_key(alias) {
            self.prosc_players.lock().unwrap().insert(
                alias.to_string(),
                // TODO: Prosc url constant
                Arc::new(Mutex::new(SequencePlayer::new(
                    "http://localhost:5000/impl/s_new",
                    output_name))
                )
            );
        }

        println!("Queue called for {}", alias);
        self.prosc_players.lock().unwrap().get(alias).unwrap().lock().unwrap()
            .queue(notes);

    }

}