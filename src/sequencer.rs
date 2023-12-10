use std::str::FromStr;
use bigdecimal::{BigDecimal, Zero}; // Floating point arithmetic is unsuitable for exact calculations 

/*

   Rewrite of sequencer code in queue.rs.
   Goal is a sequencer class that is:
       1. Isolated and well-tested
       2. Generic (entries use <beat: float, T>)
       3. Transparent (minimal amount of mutation during regular operations - e.g. don't erase the active sequence)


    GENERAL NOTE ON OVERSHOOT
        - When looping a single sequencer, the overshoot is simple to justify and calculate: 
            - You don't want to lose any progress towards the first note from the last 
        - When dealing with many sequencers, it's a lot trickier. 
            - If several of them are waiting for a final one to complete, the overshoot is effectively that of the 
                final sequencer for ALL OTHER SEQUENCERS AS WELL; the others cannot use their own overshoot values 

    NOTE ON NEXT STEPS: 
        - I believe "started" is unneeded - a higher level handler can keep a list of started vs unstarted. 

*/

#[derive(Debug, Clone)]
pub struct SequencerEntry<T: Clone> {
    pub trigger_beat: BigDecimal,
    pub contents: T,
}

pub struct Sequencer<T: Clone> {
    pub active_sequence: Vec<SequencerEntry<T>>,
    pub queued_sequence: Vec<SequencerEntry<T>>,
    pub current_beat: BigDecimal,
    pub processed_beats: Option<BigDecimal>,
    pub end_beat: BigDecimal,
    pub queue_end_beat: BigDecimal
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
        &self.current_beat >= &self.end_beat
    }

    // Use to check by how much tick() has pushed current_beat past end_beat, if at all. Only relevant if is_finished(). 
    pub fn get_overshoot(&self) -> BigDecimal {
        return match &self.current_beat > &self.end_beat {true => &self.current_beat - &self.end_beat, false => BigDecimal::zero()}; 
    }

    /*
        NOTE: This presumes that new_queue already has its entries arranged by trigger_beat rather than beats_reserved, which was common in 
            an earlier implementation. 
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