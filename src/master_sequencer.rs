/*

    Rewrite of "SequenceHandler" in queue.rs with intent similar to sequencer.rs. 




*/

use std::{collections::HashMap, hash::Hash};

use bigdecimal::BigDecimal;

use crate::sequencer::Sequencer;


/* 
    Manages a set of underlying sequencers. 
*/
pub struct MasterSequencer<T: Clone> {
    pub active_sequencers: HashMap<String, Sequencer<T>>,
    pub inactive_sequencers: HashMap<String, Sequencer<T>>,
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



}