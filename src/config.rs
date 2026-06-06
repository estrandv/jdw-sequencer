use std::path::Path;
use std::sync::OnceLock;

use log::LevelFilter;
use serde::Deserialize;
use toml::Value as TomlValue;

static CONFIG: OnceLock<Config> = OnceLock::new();
static APP_NAME: &str = "sequencer";

#[derive(Deserialize)]
pub struct Config {
    pub log_level: String,
    pub application_ip: String,
    pub application_in_port: i32,
    pub application_out_port: i32,
    pub application_out_socket_port: i32,
    pub tick_time_us: u64,
    pub sequencer_start_mode: i32,
    pub sequencer_reset_mode: i32,
    pub real_time_mode: bool,
    pub midi_sync: bool,
    pub ringbuf_capacity: usize,
    pub default_bpm: i32,
    pub buffer_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            log_level: "info".to_string(),
            application_ip: "127.0.0.1".to_string(),
            application_in_port: 14441,
            application_out_port: 13339,
            application_out_socket_port: 14444,
            tick_time_us: 5000,
            sequencer_start_mode: 1,
            sequencer_reset_mode: 1,
            real_time_mode: true,
            midi_sync: false,
            ringbuf_capacity: 100,
            default_bpm: 120,
            buffer_size: 333072,
        }
    }
}

impl Config {
    pub fn get() -> &'static Config {
        CONFIG.get().expect("Config not initialized — call config::init() first")
    }

    pub fn log_level_filter(&self) -> LevelFilter {
        match self.log_level.to_lowercase().as_str() {
            "off" | "disable" => LevelFilter::Off,
            "error" => LevelFilter::Error,
            "warn" | "warning" => LevelFilter::Warn,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            _ => LevelFilter::Info,
        }
    }
}

fn central_config_path() -> Option<String> {
    if let Ok(path) = std::env::var("JDW_CONFIG") {
        if Path::new(&path).exists() {
            return Some(path);
        }
    }
    let home = std::env::var("HOME").ok()?;
    let xdg = Path::new(&home).join(".config").join("jdw.toml");
    if xdg.exists() {
        return Some(xdg.to_string_lossy().to_string());
    }
    None
}

fn load_central_section() -> Option<TomlValue> {
    let path = central_config_path()?;
    let contents = std::fs::read_to_string(path).ok()?;
    let root: TomlValue = contents.parse().ok()?;
    root.get(APP_NAME).cloned()
}

fn merge_str(base: &mut String, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_str()) {
        *base = v.to_string();
    }
}

fn merge_i32(base: &mut i32, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_integer()) {
        *base = v as i32;
    }
}

fn merge_u64(base: &mut u64, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_integer()) {
        *base = v as u64;
    }
}

fn merge_usize(base: &mut usize, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_integer()) {
        *base = v as usize;
    }
}

fn merge_bool(base: &mut bool, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_bool()) {
        *base = v;
    }
}

fn merge_config(base: &mut Config, overlay: &TomlValue) {
    merge_str(&mut base.log_level, overlay, "log_level");
    merge_str(&mut base.application_ip, overlay, "application_ip");
    merge_i32(&mut base.application_in_port, overlay, "application_in_port");
    merge_i32(&mut base.application_out_port, overlay, "application_out_port");
    merge_i32(&mut base.application_out_socket_port, overlay, "application_out_socket_port");
    merge_u64(&mut base.tick_time_us, overlay, "tick_time_us");
    merge_i32(&mut base.sequencer_start_mode, overlay, "sequencer_start_mode");
    merge_i32(&mut base.sequencer_reset_mode, overlay, "sequencer_reset_mode");
    merge_bool(&mut base.real_time_mode, overlay, "real_time_mode");
    merge_bool(&mut base.midi_sync, overlay, "midi_sync");
    merge_usize(&mut base.ringbuf_capacity, overlay, "ringbuf_capacity");
    merge_i32(&mut base.default_bpm, overlay, "default_bpm");
    merge_usize(&mut base.buffer_size, overlay, "buffer_size");
}

impl Config {
    pub fn init(config_path: &str) {
        let mut cfg = Config::default();

        if let Some(central) = load_central_section() {
            merge_config(&mut cfg, &central);
        }

        if let Ok(contents) = std::fs::read_to_string(config_path) {
            if let Ok(local) = toml::from_str::<TomlValue>(&contents) {
                merge_config(&mut cfg, &local);
            }
        } else {
            eprintln!("Warning: Config file '{}' not found. Using defaults.", config_path);
        }

        CONFIG.set(cfg).ok();
    }
}

pub fn get_addr(port: i32) -> String {
    format!("{}:{}", Config::get().application_ip, port)
}
