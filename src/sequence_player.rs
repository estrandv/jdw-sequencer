use crate::model::sequencer::SequencerNote;
use std::cell::{Cell, RefCell};
use crate::model::rest_input::RestInputNote;
use chrono::*;
use std::borrow::Borrow;
use std::slice::SliceIndex;
use crate::model::midi_utils::beats_to_milli_seconds;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct SequencePlayer {
    // SEE: https://stackoverflow.com/questions/47748091/how-can-i-make-only-certain-struct-fields-mutable
    last_current_set_end_note_time: Cell<f32>, // Cell provides mutability for copy-implementing inner elements
    loop_start_time: Cell<DateTime<Utc>>,
    // Needs both mutable and immutable references during shift_queue()
    current_notes: Arc<Mutex<RefCell<Vec<SequencerNote>>>>,
    // Needs to be internally mutable with non-copying inner elements
    queued_notes: Arc<Mutex<RefCell<Vec<RestInputNote>>>>
}

impl SequencePlayer {

    pub fn new() -> SequencePlayer {
        SequencePlayer {
            last_current_set_end_note_time: Cell::new(0.0),
            loop_start_time: Cell::new(chrono::offset::Utc::now()),
            current_notes: Arc::new((Mutex::new(RefCell::new(Vec::new())))),
            queued_notes: Arc::new(Mutex::new(RefCell::new(Vec::new())))
        }
    }

    pub fn queue(&self, new_notes: Vec<RestInputNote>) {
        self.queued_notes.lock().unwrap().replace(new_notes);
    }

    pub fn shift_queue(&self) {
        self.current_notes.lock().unwrap().replace(Vec::new());

        let mut beat: f32 = self.last_current_set_end_note_time.get().clone();
        for note in self.queued_notes.lock().unwrap().clone().into_inner() {
            let new_note = SequencerNote {
                tone: note.tone,
                amplitude: note.amplitude,
                sustain: note.sustain_time,
                startBeat: beat
            };

            self.current_notes.lock().unwrap().get_mut().push(new_note);

            beat += note.reserved_time;
        }

        // Next time we load queued notes, use the last reserved time of
        // this set to create a starting point (so that the last note gets
        // time to finish)
        self.last_current_set_end_note_time.set(
            match self.queued_notes.lock().unwrap().clone().into_inner().last() {
                Some(last_note) => last_note.reserved_time,
                None => 0.0
            }
        )
    }

    pub fn get_next(&self, at_time: DateTime<Utc>, bpm: i32) -> Vec<SequencerNote> {

        // Note: I haven no idea how to use rc+refcell
        // https://rust-unofficial.github.io/too-many-lists/fourth-breaking.html
        // I feel some of these steps should be implicit but oh well...
        // UPDATE: Saving comment but we're using arc/mutex now
        if self.current_notes.lock().unwrap().clone().into_inner().is_empty(){
            self.shift_queue();
            self.loop_start_time.set(at_time);
        }

        let candidates = self.current_notes.lock().unwrap()
            .clone()
            .into_inner()
            .into_iter()
            .filter(|note| {
                let start = beats_to_milli_seconds(note.startBeat, bpm);
                // TODO: The i64 conversion might cause a nasty "everything at once" bug
                let note_time = self.loop_start_time.get() + Duration::milliseconds(start);
                // Not 100% sure .time() is what we're looking for as "isBefore" replacement
                note_time.time() <= at_time.time()
            }).collect::<Vec<SequencerNote>>();

        // Keep only non-candidates; the list shrinks with each call
        // Since candidates is a copied set I cannot simply do a "contains(e)" check for filtering,
        // so instead I keep only the elements with a different start time than any listed in candidates
        self.current_notes.lock().unwrap().get_mut().retain(|e| {
            !candidates.clone().into_iter().any(|e1| e1.startBeat == e.startBeat)
        });

        candidates
    }
}