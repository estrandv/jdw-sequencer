use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RestInputNote {
    pub tone: i32,
    pub reserved_time: f32,
    pub sustain_time: f32,
    pub amplitude: f32
}

/*
ALSO: the sequence data
serializable
    val notes: List<RestInputNote>
 */