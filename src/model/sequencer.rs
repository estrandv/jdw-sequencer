/*
        val tone: Double,
        val amplitude: Double,
        val sustain: Double,
        val startBeat: Double
 */
#[derive(Debug, Clone, Copy)]
pub struct SequencerNote {
    pub tone: i32,
    pub amplitude: f32,
    pub sustain: f32,
    pub startBeat: f32
}