use std::cell::RefCell;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{midi_utils};
use std::collections::HashMap;


/*
    Message sent at the start of each new full sequencer loop
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopStartMessage {
    pub time: String, // standard format date string
    pub bpm: i32
}

/*
    Message for wiping the sequence with the given alias
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequencerWipeMessage {
    pub alias: String
}

// TODO: We want to send a message with a time-stamp
// Issue is that we are typically unaware of the message contents
// So either we break that rule or start wrapping a second layer
// Wrapping a second layer means we probably have to rework all receivers
// Breaking the rule feels idiotic since it defeats the purpose of having wrapped initial messages
// We could also rework the message format, so that messages always contain timestamps
// Something like <tag>::<time>::<msg>
// ... which is probably the best way to go about it but also the most laboursome





