/*

    Rewrite of "SequenceHandler" in queue.rs with intent similar to sequencer.rs. 




*/

use std::{collections::HashMap, hash::Hash};

use bigdecimal::BigDecimal;

use crate::sequencer::{Sequencer, SequencerEntry};


/* 
    Manages a set of underlying sequencers. 
*/
pub struct MasterSequencer<T: Clone> {
    active_sequencers: HashMap<String, Sequencer<T>>,
    inactive_sequencers: HashMap<String, Sequencer<T>>,
}

impl<T: Clone> MasterSequencer<T> {
    pub fn new() -> MasterSequencer<T> {
        MasterSequencer { 
            active_sequencers: HashMap::new(),
            inactive_sequencers: HashMap::new(), 
        }
    }

    pub fn tick(&mut self, beats: BigDecimal) -> Vec<T> {
        self.active_sequencers.iter_mut()
            .flat_map(|seq| seq.1.tick(beats.clone()))
            .collect()           
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

    fn start_all_ready(&mut self) {

        // match start_mode ... 

    }

}

mod tests {
    use std::str::FromStr;

    use bigdecimal::BigDecimal;

    use crate::sequencer::SequencerEntry;

    use super::MasterSequencer;


    #[test]
    fn create_or_find_queue_test() {

        let mut ms: MasterSequencer<&str> = MasterSequencer::new();
        let entries: Vec<SequencerEntry<&str>> = vec![];

        ms.queue("one", entries.clone(), BigDecimal::from_str("0.0").unwrap());
        assert_eq!(1, ms.inactive_sequencers.len());
        ms.queue("one", entries.clone(), BigDecimal::from_str("0.0").unwrap());
        assert_eq!(1, ms.inactive_sequencers.len());
        ms.queue("two", entries.clone(), BigDecimal::from_str("0.0").unwrap());
        assert_eq!(2, ms.inactive_sequencers.len());

    }
 }
