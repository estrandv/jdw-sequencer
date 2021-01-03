use crate::model::sequencer::SequencerNote;
use std::cell::{Cell, RefCell};
use crate::model::rest_input::RestInputNote;
use chrono::*;
use std::slice::SliceIndex;
use crate::model::midi_utils::beats_to_milli_seconds;
use std::sync::{Arc, Mutex};

/*
    In school we were taught not to create behaviours based on flags when polymorphism was available.
    In rust I learned to take shortcuts I can live with...
 */
pub enum PlayerTarget {
    PROSC,
    MIDI,
    PROSC_SAMPLE
}

/*
    Keeps track of an ordered set of notes to be played at relative times.
    Core method is get_next() - see inline comments.
 */
pub struct SequencePlayer {
    // SEE: https://stackoverflow.com/questions/47748091/how-can-i-make-only-certain-struct-fields-mutable
    loop_start_time: Cell<DateTime<Utc>>,
    // Needs both mutable and immutable references during shift_queue()
    current_notes: Arc<Mutex<RefCell<Vec<SequencerNote>>>>,
    // Needs to be internally mutable with non-copying inner elements
    queued_notes: Arc<Mutex<RefCell<Vec<RestInputNote>>>>,
    pub target_output: Arc<Mutex<String>>,
    pub player_target: PlayerTarget
}

impl SequencePlayer {

    pub fn new(player_target: PlayerTarget, target_output: &str) -> SequencePlayer {
        SequencePlayer {
            loop_start_time: Cell::new(chrono::offset::Utc::now()),
            current_notes: Arc::new((Mutex::new(RefCell::new(Vec::new())))),
            queued_notes: Arc::new(Mutex::new(RefCell::new(Vec::new()))),
            target_output: Arc::new(Mutex::new(target_output.to_string())),
            player_target
        }
    }

    // Check if the player is ready to reset
    pub fn is_finished(&self) -> bool {
        self.current_notes.lock().unwrap().borrow().is_empty()
    }

    pub fn queued_time(&self) -> f32 {
        self.queued_notes.lock().unwrap().clone()
            .into_inner()
            .iter()
            .map(|note| note.reserved_time)
            .sum()
    }

    pub fn queue(&self, new_notes: Vec<RestInputNote>) {
        self.queued_notes.lock().unwrap().replace(new_notes);
    }

    /*
        Clear all current notes, calculate start times for the queued ones
            and shift those new notes into current.
     */
    pub fn shift_queue(&self, at_time: DateTime<Utc>) {
        self.current_notes.lock().unwrap().replace(Vec::new());

        let mut beat: f32 = 0.0;
        for note in self.queued_notes.lock().unwrap().clone().into_inner() {
            let new_note = SequencerNote {
                tone: note.tone,
                amplitude: note.amplitude,
                sustain: note.sustain_time,
                start_beat: beat
            };

            self.current_notes.lock().unwrap().get_mut().push(new_note);

            beat += note.reserved_time;

        }

        /*
            Since we gradually remove notes from the current set as they are played
                and notes are played based on their relative position to the previous one,
                we need a final "ghost note" to be played at the end time of the second last
                one for is_finished() to work correctly.

                TODO: This statement can be simplified
         */
        match self.queued_notes.lock().unwrap().clone().into_inner().last() {
            Some(_) => {
                let new_note = SequencerNote {
                    tone: 0.0,
                    amplitude: 0.0,
                    sustain: 0.0,
                    start_beat: beat
                };

                self.current_notes.lock().unwrap().get_mut().push(new_note);
            },
            None => ()
        }

        self.loop_start_time.set(at_time);
    }

    pub fn get_next(&self, at_time: DateTime<Utc>, bpm: i32) -> Vec<SequencerNote> {

        // Nothing to do, stall...
        if self.is_finished() {
            return Vec::new();
        }

        let candidates = self.current_notes.lock().unwrap()
            .clone()
            .into_inner()
            .into_iter()
            .filter(|note| {
                let start = beats_to_milli_seconds(note.start_beat, bpm);
                // TODO: The i64 conversion might cause a nasty "everything at once" bug
                let note_time = self.loop_start_time.get() + Duration::milliseconds(start);
                // Not 100% sure .time() is what we're looking for as "isBefore" replacement
                note_time.time() <= at_time.time()
            }).collect::<Vec<SequencerNote>>();

        // Keep only non-candidates; the list shrinks with each call
        // Since candidates is a copied set I cannot simply do a "contains(e)" check for filtering,
        // so instead I keep only the elements with a different start time than any listed in candidates
        self.current_notes.lock().unwrap().get_mut().retain(|e| {
            !candidates.clone().into_iter().any(|e1| e1.start_beat == e.start_beat)
        });

        candidates
    }
}


#[cfg(test)]
mod tests {
    use crate::sequence_player::{SequencePlayer, PlayerTarget};
    use crate::model::rest_input::RestInputNote;
    use chrono::DateTime;
    use std::time::Duration;

    #[test]
    fn sequence_length() {
        let player = SequencePlayer::new(PlayerTarget::PROSC, "none");
        player.queue(vec!(
            RestInputNote::new(1.0, 1.0, 1.0, 1.0),
            RestInputNote::new(1.0, 0.4, 1.0, 1.0),
            RestInputNote::new(1.0, 0.5, 1.0, 1.0),
            RestInputNote::new(1.0, 0.2, 1.0, 1.0),
            RestInputNote::new(1.0, 0.2, 1.0, 0.0),
        ));

        let time = chrono::offset::Utc::now();

        player.shift_queue(time);

        let mut start_times: Vec<f32> = Vec::new();
        let mut i: f32 = 0.0;
        while i < 10.0 {
            print!("MS: {}\n", i);
            let current = time + chrono::Duration::milliseconds((i * 1000.0) as i64);

            if !player.get_next(current, 60).is_empty() {
                start_times.push(i);
            }

            i = i + 0.1000000000;
        }

        assert_eq!(vec!(0.0, 1.0, 1.4, 1.9, 2.1, 2.3), start_times);
    }
}