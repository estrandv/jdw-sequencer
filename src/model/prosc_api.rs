use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SNewMessage {
    synth: String,
    values: Vec<OSCValueField>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OSCValueField {
    name: String,
    value: f32
}

impl OSCValueField { pub fn new(name: &str, value: f32) -> OSCValueField { OSCValueField { name: name.to_string(), value } } }
impl SNewMessage { pub fn new(synth: String, values: Vec<OSCValueField>) -> SNewMessage {SNewMessage{synth, values}} }