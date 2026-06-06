use std::path::Path;
use std::sync::OnceLock;

use log::LevelFilter;
use serde::Deserialize;

static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Deserialize)]
pub struct Config {
    #[serde(deserialize_with = "deserialize_log_level", default = "default_log_level_str")]
    pub log_level: LevelFilter,
    #[serde(default = "default_application_ip")]
    pub application_ip: String,
    #[serde(default = "default_application_in_port")]
    pub application_in_port: i32,
    #[serde(default = "default_application_out_port")]
    pub application_out_port: i32,
    #[serde(default = "default_application_out_socket_port")]
    pub application_out_socket_port: i32,
    #[serde(default = "default_tick_time_us")]
    pub tick_time_us: u64,
    #[serde(default = "default_sequencer_start_mode")]
    pub sequencer_start_mode: i32,
    #[serde(default = "default_sequencer_reset_mode")]
    pub sequencer_reset_mode: i32,
    #[serde(default = "default_real_time_mode")]
    pub real_time_mode: bool,
    #[serde(default = "default_midi_sync")]
    pub midi_sync: bool,
    #[serde(default = "default_ringbuf_capacity")]
    pub ringbuf_capacity: usize,
    #[serde(default = "default_default_bpm")]
    pub default_bpm: i32,
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,
}

fn default_log_level_str() -> LevelFilter {
    parse_log_level("info")
}

fn default_application_ip() -> String { "127.0.0.1".to_string() }
fn default_application_in_port() -> i32 { 14441 }
fn default_application_out_port() -> i32 { 13339 }
fn default_application_out_socket_port() -> i32 { 14444 }
fn default_tick_time_us() -> u64 { 5000 }
fn default_sequencer_start_mode() -> i32 { 1 }
fn default_sequencer_reset_mode() -> i32 { 1 }
fn default_real_time_mode() -> bool { true }
fn default_midi_sync() -> bool { false }
fn default_ringbuf_capacity() -> usize { 100 }
fn default_default_bpm() -> i32 { 120 }
fn default_buffer_size() -> usize { 333072 }

pub fn parse_log_level(s: &str) -> LevelFilter {
    match s.to_lowercase().as_str() {
        "off" | "disable" => LevelFilter::Off,
        "error" => LevelFilter::Error,
        "warn" | "warning" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    }
}

fn deserialize_log_level<'de, D: serde::Deserializer<'de>>(d: D) -> Result<LevelFilter, D::Error> {
    let s = String::deserialize(d)?;
    Ok(parse_log_level(&s))
}

impl Config {
    pub fn init(path: &str) {
        let cfg = match Path::new(path).try_exists() {
            Ok(true) => {
                let content = std::fs::read_to_string(path).unwrap_or_default();
                toml::from_str(&content).unwrap_or_else(|e| {
                    eprintln!("Warning: Failed to parse config file '{}': {}. Using defaults.", path, e);
                    Config::default()
                })
            }
            _ => {
                eprintln!("Warning: Config file '{}' not found. Using defaults.", path);
                Config::default()
            }
        };
        CONFIG.set(cfg).ok();
    }

    pub fn get() -> &'static Config {
        CONFIG.get().expect("Config not initialized — call config::init() first")
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            log_level: default_log_level_str(),
            application_ip: default_application_ip(),
            application_in_port: default_application_in_port(),
            application_out_port: default_application_out_port(),
            application_out_socket_port: default_application_out_socket_port(),
            tick_time_us: default_tick_time_us(),
            sequencer_start_mode: default_sequencer_start_mode(),
            sequencer_reset_mode: default_sequencer_reset_mode(),
            real_time_mode: default_real_time_mode(),
            midi_sync: default_midi_sync(),
            ringbuf_capacity: default_ringbuf_capacity(),
            default_bpm: default_default_bpm(),
            buffer_size: default_buffer_size(),
        }
    }
}

pub fn get_addr(port: i32) -> String {
    format!("{}:{}", Config::get().application_ip, port)
}
