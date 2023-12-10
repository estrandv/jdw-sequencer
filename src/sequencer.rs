/*

   Rewrite of sequencer code in queue.rs.
   Goal is a sequencer class that is:
       1. Isolated and well-tested
       2. Generic (entries use <beat: float, T>)
       3. Transparent (minimal amount of mutation during regular operations - e.g. don't erase the active sequence)

*/

#[derive(Debug, Clone, Copy)]
pub struct SequencerEntry<T: Clone> {
    pub beat: f32,
    pub contents: T,
}

pub struct Sequencer<T: Clone> {
    pub active_sequence: Vec<SequencerEntry<T>>,
    pub queued_sequence: Vec<SequencerEntry<T>>,
    pub current_beat: f32,
    pub processed_beats: Option<f32>,
    pub end_beat: f32,
}

impl<T: Clone> Sequencer<T> {

    pub fn reset(&mut self) {
        self.current_beat = 0.0;
        self.processed_beats = None; 
        self.active_sequence = self.queued_sequence.clone();
        // TODO: See how shift_queue handles end_beat  
    }

    pub fn tick(&mut self, beats: f32) -> Vec<T> {


        // Finished sequences stop ticking
        if !&self.is_finished() {

            self.current_beat += beats;
    
            let candidates = self
                .active_sequence
                .iter()
                .filter(|n| {
                    &n.beat <= &self.current_beat && match &self.processed_beats { Some(value) => &n.beat > value, None => true }
                })
                .map(|n| n.clone().contents.clone())
                .collect();
    
            // Note that entries up until this beat have been tick-returned and should not be returned again on later current_beats
            self.processed_beats = Some(self.current_beat);
    
            candidates
    
        } else {
            vec![]
        }

    }

    pub fn is_finished(&self) -> bool {
        &self.current_beat >= &self.end_beat
    }



}

mod tests {
    use super::SequencerEntry;
    use super::Sequencer;

    #[test]
    fn tick_test() {
        let entries: Vec<SequencerEntry<&str>> = vec![
            SequencerEntry {beat: 0.0, contents:"one"},    
            SequencerEntry {beat: 0.5, contents:"two"},    
            SequencerEntry {beat: 1.5, contents:"three"},    
        ];

        let mut sequencer: Sequencer<&str> = Sequencer {
            active_sequence: entries.clone(),
            queued_sequence: entries.clone(),
            current_beat: 0.0,
            processed_beats: None,
            end_beat: 3.0
        };

        assert_eq!(sequencer.tick(0.25), vec!["one"]);
        assert_eq!(sequencer.current_beat, 0.25);
        assert_eq!(sequencer.processed_beats, Some(0.25));

        assert_eq!(sequencer.tick(0.25), vec!["two"]);
        assert_eq!(sequencer.current_beat, 0.5);
        assert_eq!(sequencer.processed_beats, Some(0.5));
        assert_eq!(sequencer.tick(0.25).is_empty(), true);
        assert_eq!(sequencer.tick(0.25).is_empty(), true);
        assert_eq!(sequencer.tick(0.25).is_empty(), true);
        assert_eq!(sequencer.tick(0.25), vec!["three"]);
        assert_eq!(sequencer.tick(1.4).is_empty(), true);
        assert_eq!(sequencer.current_beat, 2.9);
        assert_eq!(sequencer.is_finished(), false);
        assert_eq!(sequencer.tick(0.3).is_empty(), true);
        assert_eq!(sequencer.current_beat, 3.2);
        assert_eq!(sequencer.is_finished(), true);
        assert_eq!(sequencer.tick(0.3).is_empty(), true);
        assert_eq!(sequencer.current_beat, 3.2);
        assert_eq!(sequencer.is_finished(), true);

    }
}