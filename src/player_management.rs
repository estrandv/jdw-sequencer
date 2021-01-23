use crate::rest::RestClient;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::sequence_player::{SequencePlayer, PlayerTarget};
use chrono::{DateTime, Utc};
use crate::model::rest_input::RestInputNote;
use std::cell::{Cell, RefCell};
use std::thread;

/*
    Organizer for sequence players targeting PROSC.
 */
pub struct PlayerManager {
    sequence_players: Arc<Mutex<HashMap<String, Arc<Mutex<SequencePlayer>>>>>,
    rest_client: Arc<Mutex<RestClient>>,
    longest_active_player_dur: Arc<Mutex<Cell<f32>>>,
    beat_counter: Arc<Mutex<Cell<f32>>>,
}

impl PlayerManager {

    pub fn new(rc: Arc<Mutex<RestClient>>) -> PlayerManager {
        PlayerManager {
            sequence_players: Arc::new(Mutex::new(HashMap::new())),
            rest_client: rc,
            longest_active_player_dur: Arc::new(Mutex::new(Cell::new(0.0))),
            beat_counter: Arc::new(Mutex::new(Cell::new(0.0))),
        }
    }

    pub fn force_reset(&self) {
        for (_, player) in self.sequence_players.lock().unwrap().iter() {
            player.lock().unwrap().shift_queue();
        }
        self.beat_counter.lock().unwrap().set(0.0);
    }

    // Scan for upcoming notes in all players and send to REST targets where appropriate
    pub fn play_next(&self, beats_elapsed: f32) {

        self.beat_counter.lock().unwrap().update(| v| v + beats_elapsed);

        // TODO: The actual sending seems to be a rather heavy operation which can result in
        //  annoying hiccups on very large player set sizes (tested on >100)
        for (name, player) in self.sequence_players.lock().unwrap().iter() {
            let notes_on_time = player.lock().unwrap().get_next(
                self.beat_counter.lock().unwrap().clone().into_inner()
            );
            if notes_on_time.len() > 1 {
                println!("WARNING: Note overflow in {} {:?}", name, notes_on_time.clone());
            }
            if !notes_on_time.is_empty() {

                let output = player.lock().unwrap().target_output.lock().unwrap().clone();
                let thread_player = player.clone();
                let thread_client = self.rest_client.clone();

                println!("Sending note on beat: {}", self.beat_counter.lock().unwrap().clone().into_inner());

                // Not sure how much this threading matters, but as I understand http ALWAYS waits for a response.
                // Since the response doesn't really matter to the sending thread, we just leave the send
                // in a thread and move on...
                thread::spawn(move || {
                    match thread_player.player_target {
                        PlayerTarget::PROSC => {
                            thread_client.lock().unwrap()
                                .post_prosc_notes( &output, notes_on_time.clone());
                        },
                        PlayerTarget::MIDI => {
                            thread_client.lock().unwrap()
                                .post_midi_notes( &output, notes_on_time.clone());
                        },
                        PlayerTarget::PROSC_SAMPLE => {
                            thread_client.unwrap()
                                .post_prosc_samples( &output, notes_on_time.clone());
                        },
                    }
                });

            }
        }

        // Immediately reset the counter if all players have finished their playing set
        let all_finished = self.sequence_players.lock().unwrap().values().into_iter()
            .all(|player| player.lock().unwrap().is_finished());

        if all_finished {
            self.force_reset();
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

        println!("Queue called for {}: {:?}", alias, notes);
        self.sequence_players.lock().unwrap().get(alias).unwrap().lock().unwrap()
            .queue(notes);

    }

}