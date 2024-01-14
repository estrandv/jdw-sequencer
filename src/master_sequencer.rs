/*

    Orchestration struct for sequencers. Mainly a composite struct to conveniently wrap list/map functions, but 
        notably also handles the business logic of start and reset rules. 

*/

use std::{collections::HashMap, hash::Hash, str::FromStr};

use bigdecimal::BigDecimal;

use crate::sequencer::{Sequencer, SequencerEntry};

pub enum SequencerResetMode {
    AllAfterLongestSequenceFinished,
    Individual
}

pub enum SequencerStartMode {
    WithNearestSequence,
    WithLongestSequence,
    Immediate
}

/* 
    Manages a set of underlying sequencers. 
*/
pub struct MasterSequencer<T: Clone> {
    active_sequencers: HashMap<String, Sequencer<T>>,
    inactive_sequencers: HashMap<String, Sequencer<T>>,
    pub sequencer_start_mode: SequencerStartMode,
    pub sequencer_reset_mode: SequencerResetMode
}

impl<T: Clone> MasterSequencer<T> {
    pub fn new(start_mode: SequencerStartMode, reset_mode: SequencerResetMode) -> MasterSequencer<T> {
        MasterSequencer { 
            active_sequencers: HashMap::new(),
            inactive_sequencers: HashMap::new(),
            sequencer_start_mode: start_mode,
            sequencer_reset_mode: reset_mode
        }
    }

    pub fn tick(&mut self, beats: BigDecimal) -> Vec<T> {
        self.active_sequencers.iter_mut()
            .flat_map(|seq| seq.1.tick(beats.clone()))
            .collect()
        
    }

    pub fn reset_check(&mut self) {

        /*
            General note on overshoot: It's not always crystal clear what the overshoot is. 
                But when several sequencers are waiting for the longest one to complete, the 
                overshoot is likely produced by the last tick on the longest sequencer, rather than
                in each sequencer individually. 
        */
        match self.sequencer_reset_mode {
            SequencerResetMode::AllAfterLongestSequenceFinished => {
                if self.longest_sequence_finished() {
                    let overshoot = self.get_longest_overshoot();
                    self.active_sequencers.iter_mut()
                        .filter(|seq| seq.1.is_finished())
                        .for_each(|seq| seq.1.reset(overshoot.clone()));
                }
            },
            SequencerResetMode::Individual => {
                self.active_sequencers.iter_mut()
                    .filter(|seq| seq.1.is_finished())
                    .for_each(|seq| {
                        let overshoot = seq.1.get_overshoot();
                        seq.1.reset(overshoot);
                    });
            },
        }

    }

    /*
        Queue the entries for the given sequencer alias, creating a new inactive sequencer if necessary
    */
    pub fn queue(&mut self, sequencer_alias: &str, entries: Vec<SequencerEntry<T>>, end_beat: BigDecimal) {
        let existing = self.active_sequencers.get_mut(sequencer_alias).or(
            self.inactive_sequencers.get_mut(sequencer_alias)
        ); 

        if existing.is_some() {
            existing.map(|seq| {
                seq.queue(entries, end_beat);
            });
        } else {
            let mut new_seq = Sequencer::new();
            new_seq.queue(entries, end_beat);
            self.inactive_sequencers.insert(sequencer_alias.to_string(), new_seq);
        }
    }

    pub fn start_check(&mut self) {

        let start_mode_ok = match self.sequencer_start_mode {
            SequencerStartMode::WithLongestSequence => self.longest_sequence_finished() || self.count_started() == 0,
            SequencerStartMode::WithNearestSequence => self.count_finished() > 0 || self.count_started() == 0,
            SequencerStartMode::Immediate => true,
        };

        let should_start = start_mode_ok && !self.inactive_sequencers.is_empty();

        if should_start {
            for entry in self.inactive_sequencers.iter() {
                self.active_sequencers.insert(entry.0.to_string(), entry.1.clone());
            }
            self.inactive_sequencers.clear();
        }

    }

    fn count_finished(&self) -> usize{
        self.active_sequencers.iter().filter(|seq| seq.1.is_finished()).count()
    }

    fn count_started(&self) -> usize {
        self.active_sequencers.capacity()
    }

    fn longest_sequence_finished(&self) -> bool {
        let longest_sequence = self.active_sequencers.iter()
            .max_by(|seq1, seq2| seq1.1.end_beat.cmp(&seq2.1.end_beat))
            .map(|seq| seq.1);

        longest_sequence.map(|seq| seq.is_finished()).unwrap_or(false)

    }

    fn get_longest_overshoot(&self) -> BigDecimal {
        let longest_sequence = self.active_sequencers.iter()
            .max_by(|seq1, seq2| seq1.1.end_beat.cmp(&seq2.1.end_beat))
            .map(|seq| seq.1);

        longest_sequence.map(|seq| seq.get_overshoot()).unwrap_or(BigDecimal::from_str("0.0").unwrap())

    }


}

mod tests {
    use std::str::FromStr;

    use bigdecimal::BigDecimal;

    use crate::{sequencer::SequencerEntry, master_sequencer::{SequencerStartMode, SequencerResetMode}};

    use super::MasterSequencer;

    fn big(inp: &str) -> BigDecimal {
        BigDecimal::from_str(inp).unwrap()
    }

    // TODO: More start mode tests 

    #[test]
    fn start_mode_longest_test() {
        let mut ms: MasterSequencer<&str> = MasterSequencer::new(SequencerStartMode::WithLongestSequence, SequencerResetMode::Individual);
        let entries1 = vec![
            SequencerEntry::new(big("0.0"), "one"),
        ];
        ms.queue("longest", entries1.clone(), big("3.0"));
        ms.start_check();
        ms.reset_check();
        ms.queue("first", entries1.clone(), big("1.0"));
        ms.queue("second", entries1.clone(), big("1.5"));
        assert_eq!(ms.active_sequencers.len(), 1);
        assert_eq!(ms.inactive_sequencers.len(), 2);
        assert_eq!(ms.count_finished(), 0);
        ms.start_check();
        assert_eq!(ms.active_sequencers.len(), 1);
        assert_eq!(ms.inactive_sequencers.len(), 2);
        assert_eq!(ms.count_finished(), 0);
        ms.reset_check();
        assert_eq!(ms.count_finished(), 0);

        ms.tick(big("1.0"));
        ms.start_check();
        ms.reset_check();
        assert_eq!(ms.active_sequencers.len(), 1);
        assert_eq!(ms.inactive_sequencers.len(), 2);
        assert_eq!(ms.count_finished(), 0);
        
        ms.tick(big("1.9"));
        ms.start_check();
        ms.reset_check();
        assert_eq!(ms.active_sequencers.len(), 1);
        assert_eq!(ms.inactive_sequencers.len(), 2);
        assert_eq!(ms.count_finished(), 0);
    
        ms.tick(big("0.1"));
        ms.start_check();
        ms.reset_check();
        assert_eq!(ms.active_sequencers.len(), 3);
        assert_eq!(ms.inactive_sequencers.len(), 0);
        assert_eq!(ms.count_finished(), 0);
        


    }

    #[test]
    fn reset_check_individual_test() {
        let mut ms: MasterSequencer<&str> = MasterSequencer::new(SequencerStartMode::Immediate, SequencerResetMode::Individual);
        let entries1 = vec![
            SequencerEntry::new(big("0.0"), "one"),
        ];

        // TODO: Paste into test that needs em
        let entries2 = vec![
            SequencerEntry::new(big("0.0"), "one"),
            SequencerEntry::new(big("0.1"), "two"),
        ];

        let entries3 = vec![
            SequencerEntry::new(big("0.0"), "one"),
            SequencerEntry::new(big("0.1"), "two"),
            SequencerEntry::new(big("0.2"), "three"),
        ];

        ms.queue("first", entries1.clone(), big("1.0"));
        ms.queue("second", entries1.clone(), big("1.5"));
        assert_eq!(ms.active_sequencers.len(), 0);
        assert_eq!(ms.inactive_sequencers.len(), 2);
        assert_eq!(ms.count_finished(), 0); 
        ms.start_check();
        assert_eq!(ms.active_sequencers.len(), 2);
        assert_eq!(ms.inactive_sequencers.len(), 0);
        assert_eq!(ms.count_finished(), 2); // Without reset, the queue end beat has not yet become the regular end beat
        ms.reset_check();
        assert_eq!(ms.count_finished(), 0);
        ms.tick(big("1.0"));
        assert_eq!(ms.count_finished(), 1); 
        ms.reset_check();
        assert_eq!(ms.count_finished(), 0); 

    }

    #[test]
    fn reset_check_longest_test() {
        let mut ms: MasterSequencer<&str> = MasterSequencer::new(SequencerStartMode::Immediate, SequencerResetMode::AllAfterLongestSequenceFinished);
        let entries1 = vec![
            SequencerEntry::new(big("0.0"), "one"),
        ];

        ms.queue("first", entries1.clone(), big("1.0"));
        ms.queue("second", entries1.clone(), big("1.5"));
        ms.queue("longest", entries1.clone(), big("3.0"));
        assert_eq!(ms.active_sequencers.len(), 0);
        assert_eq!(ms.inactive_sequencers.len(), 3);
        assert_eq!(ms.count_finished(), 0); 
        ms.start_check();
        assert_eq!(ms.active_sequencers.len(), 3);
        assert_eq!(ms.inactive_sequencers.len(), 0);
        assert_eq!(ms.count_finished(), 3); // Without reset, the queue end beat has not yet become the regular end beat
        ms.reset_check();
        assert_eq!(ms.count_finished(), 0);
        ms.tick(big("1.0"));
        assert_eq!(ms.count_finished(), 1); 
        ms.reset_check();
        assert_eq!(ms.count_finished(), 1); 
        ms.tick(big("1.0"));
        assert_eq!(ms.count_finished(), 2);
        ms.reset_check();
        assert_eq!(ms.count_finished(), 2);
        ms.tick(big("1.2")); // Here, the longest finishes with an overshoot of 0.2 
        assert_eq!(ms.count_finished(), 3);
        ms.reset_check();
        assert_eq!(ms.count_finished(), 0);

        for sequence in ms.active_sequencers.iter() {
            assert_eq!(sequence.1.current_beat, big("0.2"));
        }
        
    }
    



    #[test]
    fn create_or_find_queue_test() {

        let mut ms: MasterSequencer<&str> = MasterSequencer::new(SequencerStartMode::Immediate, SequencerResetMode::Individual);
        let entries: Vec<SequencerEntry<&str>> = vec![];
        ms.queue("one", entries.clone(), big("0.0"));
        assert_eq!(1, ms.inactive_sequencers.len());
        ms.queue("one", entries.clone(), big("0.0"));
        assert_eq!(1, ms.inactive_sequencers.len());
        ms.queue("two", entries.clone(), big("0.0"));
        assert_eq!(2, ms.inactive_sequencers.len());

    }
 }
