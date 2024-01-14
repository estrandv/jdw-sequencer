use std::str::FromStr;
use bigdecimal::{BigDecimal, Zero};
use log::{debug, info};


/*

    NOTE: Rewrite of code in queue.rs. 

    Generic sequencer class.
    See description of fields in struct. 

*/

#[derive(Debug, Clone)]
pub struct SequencerEntry<T: Clone> {
    pub trigger_beat: BigDecimal,
    pub contents: T,
}

impl<T: Clone> SequencerEntry<T> {
    pub fn new(beat: BigDecimal, entry: T) -> SequencerEntry<T> {
        SequencerEntry {
            trigger_beat: beat,
            contents: entry
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sequencer<T: Clone> {
    pub active_sequence: Vec<SequencerEntry<T>>, // The current sequence, accessed with tick() until end_beat is reached. 
    pub queued_sequence: Vec<SequencerEntry<T>>, // The queued sequence that will replace active_sequence when end_beat is reached. 
    pub current_beat: BigDecimal, // Beat timeline, to be compared with end_beat. 
    processed_beats: Option<BigDecimal>, // Last step of current_beat, used for internal logic. 
    pub end_beat: BigDecimal, // The last beat of the current sequence. 
    pub queue_end_beat: BigDecimal // The last beat of the queued sequence. Replaces end_beat when end_beat is reached. 
}

impl<T: Clone> Sequencer<T> {

    pub fn new() -> Sequencer<T> {
        Sequencer {
            active_sequence: vec![],
            queued_sequence: vec![],
            current_beat: BigDecimal::zero(),
            processed_beats: None,
            end_beat: BigDecimal::zero(),
            queue_end_beat: BigDecimal::zero()
        }
    }

    // Overshoot is the amount of beats you've already started counting on the new sequence, e.g. by having the last tick amount
    //  overshoot the end_beat by n amount
    pub fn reset(&mut self, overshoot: BigDecimal) {
        self.current_beat = overshoot;
        self.processed_beats = None; 
        self.active_sequence = self.queued_sequence.clone();
        self.end_beat = self.queue_end_beat.clone();
    }

    pub fn tick(&mut self, beats: BigDecimal) -> Vec<T> {


        // Finished sequences stop ticking
        if !&self.is_finished() {

            self.current_beat += beats;
    
            let candidates = self
                .active_sequence
                .iter()
                .filter(|n| {
                    // Bit chunky! In rough terms: entries not yet processed which current_beat has now passed. 
                    &n.trigger_beat <= &self.current_beat && match &self.processed_beats { Some(value) => &n.trigger_beat > value, None => true }
                })
                .map(|n| n.clone().contents.clone())
                .collect();
    
            // Note that entries up until this beat have been tick-returned and should not be returned again on later current_beats
            self.processed_beats = Some(self.current_beat.clone());
    
            candidates
    
        } else {
            vec![]
        }

    }

    pub fn is_finished(&self) -> bool {
        let cursor = &self.processed_beats.clone().unwrap_or(self.current_beat.clone());
        return cursor >= &self.end_beat; 
    }

    // Use to check by how much tick() has pushed current_beat past end_beat, if at all. Only relevant if is_finished(). 
    pub fn get_overshoot(&self) -> BigDecimal {
        return match &self.current_beat > &self.end_beat {true => &self.current_beat - &self.end_beat, false => BigDecimal::zero()}; 
    }

    /*
        NOTE: This presumes that new_queue already has its entries arranged on a timeline, with each time signature representing their start time. 
    */
    pub fn queue(&mut self, new_queue: Vec<SequencerEntry<T>>, end_beat: BigDecimal) {
        self.queue_end_beat = end_beat.clone();
        self.queued_sequence = new_queue.clone();
    }


}

mod tests {
    use std::str::FromStr;

    use super::SequencerEntry;
    use super::Sequencer;
    use bigdecimal::BigDecimal;

    #[test]
    fn reset_test() {
        let entries: Vec<SequencerEntry<&str>> = vec![
            SequencerEntry {trigger_beat: BigDecimal::from_str("0.0").unwrap(), contents:"one"},    
            SequencerEntry {trigger_beat: BigDecimal::from_str("0.2").unwrap(), contents:"two"},    
            SequencerEntry {trigger_beat: BigDecimal::from_str("1.0").unwrap(), contents:"three"},    
        ];

        let mut sequencer = Sequencer::new();
        sequencer.queue(entries, big("1.5"));
        sequencer.reset(big("0.3"));

        assert_eq!(sequencer.current_beat, BigDecimal::from_str("0.3").unwrap());
        assert_eq!(sequencer.processed_beats, None);

        assert_eq!(sequencer.tick(big("0.6")), vec!["one", "two"]);
        assert_eq!(sequencer.current_beat, BigDecimal::from_str("0.9").unwrap());
        assert_eq!(&sequencer.processed_beats.clone().unwrap(), &BigDecimal::from_str("0.9").unwrap());

    }

    fn big(inp: &str) -> BigDecimal {
        BigDecimal::from_str(inp).unwrap()
    }

    #[test]
    fn tick_test() {
        let entries: Vec<SequencerEntry<&str>> = vec![
            SequencerEntry {trigger_beat: big("0.0"), contents:"one"},    
            SequencerEntry {trigger_beat: big("0.5"), contents:"two"},    
            SequencerEntry {trigger_beat: big("1.5"), contents:"three"},    
        ];

        let mut sequencer = Sequencer::new();
        sequencer.queue(entries, big("3.0"));
        sequencer.reset(big("0.0"));

        assert_eq!(sequencer.tick(big("0.25")), vec!["one"]);
        assert_eq!(sequencer.current_beat, big("0.25"));
        assert_eq!(sequencer.processed_beats.clone().unwrap(), big("0.25"));

        assert_eq!(sequencer.tick(big("0.25")), vec!["two"]);
        assert_eq!(sequencer.current_beat, big("0.5"));
        assert_eq!(sequencer.processed_beats.clone().unwrap(), big("0.5"));
        assert_eq!(sequencer.tick(big("0.25")).is_empty(), true);
        assert_eq!(sequencer.tick(big("0.25")).is_empty(), true);
        assert_eq!(sequencer.tick(big("0.25")).is_empty(), true);
        assert_eq!(sequencer.tick(big("0.25")), vec!["three"]);
        assert_eq!(sequencer.tick(big("1.4")).is_empty(), true);
        assert_eq!(sequencer.current_beat, big("2.9"));
        assert_eq!(sequencer.is_finished(), false);
        assert_eq!(sequencer.tick(big("0.3")).is_empty(), true);
        assert_eq!(sequencer.current_beat, big("3.2"));
        assert_eq!(sequencer.is_finished(), true);
        assert_eq!(sequencer.tick(big("0.3")).is_empty(), true);
        assert_eq!(sequencer.current_beat, big("3.2"));
        assert_eq!(sequencer.is_finished(), true);

    }
}