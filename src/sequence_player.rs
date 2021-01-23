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
    current_notes: Arc<Mutex<RefCell<Vec<SequencerNote>>>>,
    // Needs to be internally mutable with non-copying inner elements
    queued_notes: Arc<Mutex<RefCell<Vec<SequencerNote>>>>,
    pub target_output: Arc<Mutex<String>>,
    pub player_target: PlayerTarget
}

impl SequencePlayer {

    pub fn new(player_target: PlayerTarget, target_output: &str) -> SequencePlayer {
        SequencePlayer {
            current_notes: Arc::new(Mutex::new(RefCell::new(Vec::new()))),
            queued_notes: Arc::new(Mutex::new(RefCell::new(Vec::new()))),
            target_output: Arc::new(Mutex::new(target_output.to_string())),
            player_target
        }
    }

    // Check if the player is ready to reset
    pub fn is_finished(&self) -> bool {
        self.current_notes.lock().unwrap().borrow().is_empty()
    }

    pub fn queue(&self, new_notes: Vec<RestInputNote>) {
        self.queued_notes.lock().unwrap().replace(Vec::new());

        let mut beat: f32 = 0.0;
        for note in new_notes {
            let new_note = SequencerNote {
                tone: note.tone,
                amplitude: note.amplitude,
                sustain: note.sustain_time,
                start_beat: beat
            };

            self.queued_notes.lock().unwrap().get_mut().push(new_note);

            beat += note.reserved_time;

        }

        /*
    Since we transform reserved time (effectively end time) to start time when shuffling
    and then use start time to trigger everything, the very last note will not get its
    play time unless we also add a dummy note at the end. This way, the sequencer will reach
    the end note, play it, empty itself and then reset, as opposed to resetting as soon as
    the second last tone is played.

    TODO: I keep returning to this as a bug source but I think it works just fine.
        HOWEVER since this SEEMS to be the issue when in actuality it's something else
        I guess we need to up our test game a bit...
 */

        if !self.queued_notes.lock().unwrap().clone().into_inner().is_empty() {
            self.queued_notes.lock().unwrap().get_mut().push(
                SequencerNote {
                    tone: 0.0,
                    amplitude: 0.0,
                    sustain: 0.0,
                    start_beat: beat
                }
            );
        }

    }

    /*
        Replace current set with queued set
     */
    pub fn shift_queue(&self) {
        self.current_notes.lock().unwrap().replace(self.queued_notes.lock().unwrap().clone().into_inner());
    }

    pub fn get_next(&self, current_beat: f32) -> Vec<SequencerNote> {

        // Nothing to do, stall...
        if self.is_finished() {
            return Vec::new();
        }

        let candidates = self.current_notes.lock().unwrap()
            .clone()
            .into_inner()
            .into_iter()
            .filter(|note| {
                note.start_beat <= current_beat
            })
            .collect::<Vec<SequencerNote>>();

        // Keep only non-candidates; the list shrinks with each call
        // Since candidates is a copied set I cannot simply do a "contains(e)" check for filtering,
        // so instead I keep only the elements with a different start time than any listed in candidates
        self.current_notes.lock().unwrap().get_mut().retain(|e| {
            !candidates.clone().into_iter().any(|e1| e1.start_beat == e.start_beat)
        });

        candidates.into_iter()
            .filter(|note| {
                note.amplitude > 0.0 // Note! For most services there is no point in playing silent notes. This chould change...
            })
            .collect::<Vec<SequencerNote>>()
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

        player.shift_queue();

        assert_eq!(player.get_next(0.0).len(), 1);
        assert!(player.get_next(0.1).is_empty());
        assert!(!player.get_next(1.0).is_empty());
        assert!(!player.get_next(1.4).is_empty());
        assert!(!player.get_next(1.9).is_empty());
        assert!(player.get_next(2.0).is_empty());
        assert!(!player.get_next(2.1).is_empty());
        assert!(!player.get_next(2.3).is_empty());
    }
}