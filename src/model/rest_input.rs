use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RestInputNote {
    pub tone: i32,
    pub reserved_time: f32,
    pub sustain_time: f32,
    pub amplitude: f32
}

impl RestInputNote {
    pub fn new(
        tone: i32,
        res: f32,
        sus: f32,
        amp: f32
    ) -> RestInputNote {
        RestInputNote {
            tone: tone,
            reserved_time: res,
            sustain_time: sus,
            amplitude: amp
        }
    }
}

/*
ALSO: the sequence data
serializable
    val notes: List<RestInputNote>
 */