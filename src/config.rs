use log::{LevelFilter};

/*
    Central place for application configuration until we decide on a non-hardcode method
 */

pub const LOG_LEVEL: LevelFilter = LevelFilter::Info;
pub const APPLICATION_IP: &str = "127.0.0.1";

pub const APPLICATION_IN_PORT: i32 = 14441; // Send messages here
pub const APPLICATION_OUT_PORT: i32 = 14443; // This is the one you listen to
pub const APPLICATION_OUT_SOCKET_PORT: i32 = 14444; // ... and this is just for reserving a non-polling socket

// "US" = Microseconds
pub const TICK_TIME_US: u64 = 2000; // 2ms?

pub fn get_addr(port: i32) -> String {
    format!("{}:{}", APPLICATION_IP, port)
}