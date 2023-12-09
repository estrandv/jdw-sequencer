use log::{LevelFilter};

/*
    Central place for application configuration until we decide on a non-hardcode method
 */

pub const LOG_LEVEL: LevelFilter = LevelFilter::Info;
pub const APPLICATION_IP: &str = "127.0.0.1";

pub const APPLICATION_IN_PORT: i32 = 14441; // Messages sent to this port will be read by this application
//pub const APPLICATION_OUT_PORT: i32 = 14443; // This application sends its outgoing messages to this port
//pub const APPLICATION_OUT_PORT: i32 = 13331; // Hardwire to jdw-sc
pub const APPLICATION_OUT_PORT: i32 = 13339; // jdw-osc-router
//pub const APPLICATION_OUT_PORT: i32 = 12367; // Hardwire to jdw-sampler

pub const APPLICATION_OUT_SOCKET_PORT: i32 = 14444; // Messages send from this application will have this port listed as "from"

// "US" = Microseconds
pub const TICK_TIME_US: u64 = 4000; // 4ms?

// TODO: Enum / own struct for these?
pub const SEQ_START_MODE_NEAREST: i32 = 0;
pub const SEQ_START_MODE_LONGEST: i32 = 1;
pub const SEQ_START_MODE_IMMEDIATE: i32 = 2;

pub const SEQUENCER_START_MODE: i32 = SEQ_START_MODE_LONGEST;

pub const SEQ_RESET_MODE_TOGETHER: i32 = 0;
pub const SEQ_RESET_MODE_INDIVIDUAL: i32 = 1;

pub const SEQUENER_RESET_MODE: i32 = SEQ_RESET_MODE_INDIVIDUAL;


pub const MIDI_SYNC: bool = false; 

pub fn get_addr(port: i32) -> String {
    format!("{}:{}", APPLICATION_IP, port)
}