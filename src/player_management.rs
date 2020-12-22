use crate::rest::RestClient;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::sequence_player::{SequencePlayer, PlayerTarget};
use chrono::{DateTime, Utc};
use crate::model::rest_input::RestInputNote;
use std::cell::{Cell, RefCell};

/*
    Organizer for sequence players targeting PROSC.
 */
pub struct PlayerManager {
    sequence_players: Arc<Mutex<HashMap<String, Arc<Mutex<SequencePlayer>>>>>,
    rest_client: Arc<Mutex<RestClient>>,
    longest_active_player_dur: Arc<Mutex<Cell<f32>>>
}

impl PlayerManager {

    pub fn new(rc: Arc<Mutex<RestClient>>) -> PlayerManager {
        PlayerManager {
            sequence_players: Arc::new(Mutex::new(HashMap::new())),
            rest_client: rc,
            longest_active_player_dur: Arc::new(Mutex::new(Cell::new(0.0))),
        }
    }

    // Scan for upcoming notes in all players and send to PROSC where appropriate
    pub fn play_next(&self, time: DateTime<Utc>, bpm: i32) {

        let all_finished = self.sequence_players.lock().unwrap().values().into_iter()
            .all(|player| player.lock().unwrap().is_finished());

        if all_finished {
            for (_, player) in self.sequence_players.lock().unwrap().iter() {
                player.lock().unwrap().shift_queue(time);
            }
        }

        for (_, player) in self.sequence_players.lock().unwrap().iter() {
            let notes_on_time = player.lock().unwrap().get_next(time, bpm);
            if notes_on_time.len() > 1 {
                println!("WARNING: Note overflow!");
            }
            if !notes_on_time.is_empty() {

                let output = player.lock().unwrap().target_output.lock().unwrap().clone();

                match player.lock().unwrap().player_target {
                    PlayerTarget::PROSC => {
                        self.rest_client.clone().lock().unwrap()
                            .post_prosc_notes( &output, notes_on_time.clone());
                    },
                    PlayerTarget::MIDI => {
                        self.rest_client.clone().lock().unwrap()
                            .post_midi_notes( &output, notes_on_time.clone());
                    }
                }

            }
        }
    }

    // Queue a set of notes for the given output name.
    // Non-existing output players will be created.
    pub fn queue_notes(&self, target: PlayerTarget, output_name: &str, alias: &str, notes: Vec<RestInputNote>) {
        if !self.sequence_players.lock().unwrap().contains_key(alias) {
            self.sequence_players.lock().unwrap().insert(
                alias.to_string(),
                Arc::new(Mutex::new(SequencePlayer::new(
                    target,
                    output_name))
                )
            );
        }

        println!("Queue called for {}", alias);
        self.sequence_players.lock().unwrap().get(alias).unwrap().lock().unwrap()
            .queue(notes);

    }

}