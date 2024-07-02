use bigdecimal::BigDecimal;

use crate::sequencer::SequencerEntry;

#[derive(Debug, Clone)]
pub enum LocalSequencerMessage<T: Clone> {
    HardStop,
    Reset,
    SetBpm(i32),
    EndAfterFinish,
    Queue(LocalQueuePayload<T>),
    BatchQueue(Vec<LocalQueuePayload<T>>),
}

#[derive(Debug, Clone)]
pub struct LocalQueuePayload<T: Clone> {
    pub sequencer_alias: String,
    pub entries: Vec<SequencerEntry<T>>,
    pub end_beat: BigDecimal, 
    pub one_shot: bool,
}