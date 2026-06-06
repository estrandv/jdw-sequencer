#![feature(result_flattening, proc_macro_hygiene, decl_macro)]

pub mod bundle_model;
pub mod config;
pub mod local_messaging;
pub mod master_sequencer;
pub mod midi_utils;
pub mod osc_communication;
pub mod sequencer;
pub mod sequencing_daemon;

use std::sync::{Arc, Mutex};

use local_messaging::{LocalQueuePayload, LocalSequencerMessage};
use log::{info, warn};
use master_sequencer::MasterSequencer;
use ringbuf::traits::{Producer, Split};
use ringbuf::HeapRb;
use rosc::{OscBundle, OscMessage, OscPacket, OscTime, OscType};
use std::convert::TryFrom;
use std::time::SystemTime;
use chrono::{DateTime, Utc};

use bundle_model::UpdateQueueMessage;

use crate::bundle_model::BatchUpdateQueuesMessage;
use crate::osc_communication::OSCClient;
use jdw_osc_lib::osc_stack::OSCStack;

/// Run the jdw-sequencer daemon. Blocks the calling thread indefinitely.
///
/// * `config_path` – path to the per-app `config.toml`.
/// * `quiet`       – suppress non-error log output.
pub fn run(config_path: &str, quiet: bool) {
    config::Config::init(config_path);

    let cfg = config::Config::get();

    // Logging must only be initialised once per process; in library mode the
    // caller is responsible. We attempt init here for the standalone-binary
    // case and silently ignore the "already initialised" error.
    let _ = simple_logger::SimpleLogger::new()
        .with_level(if quiet {
            log::LevelFilter::Error
        } else {
            cfg.log_level_filter()
        })
        .init();

    let osc_pipe = HeapRb::<LocalSequencerMessage<OscPacket>>::new(cfg.ringbuf_capacity);
    let (osc_pub, osc_sub) = osc_pipe.split();

    let osc_pub_mutex = Arc::new(Mutex::new(osc_pub));

    let osc_client = OSCClient::new();

    let start_mode = match cfg.sequencer_start_mode {
        0 => master_sequencer::SequencerStartMode::WithNearestSequence,
        2 => master_sequencer::SequencerStartMode::Immediate,
        _ => master_sequencer::SequencerStartMode::WithLongestSequence,
    };
    let reset_mode = match cfg.sequencer_reset_mode {
        0 => master_sequencer::SequencerResetMode::AllAfterLongestSequenceFinished,
        _ => master_sequencer::SequencerResetMode::Individual,
    };

    let master = MasterSequencer::new(start_mode, reset_mode);

    sequencing_daemon::start_live_loop::<OscPacket, _>(
        master,
        cfg.default_bpm,
        osc_sub,
        move |packets_to_send, tick_time| {
            if !packets_to_send.is_empty() {
                info!("TICK! {:?}", tick_time);

                let send_packets = packets_to_send.iter().map(|pct| {
                    if cfg.real_time_mode {
                        OscPacket::Bundle(OscBundle {
                            timetag: OscTime::try_from(tick_time).unwrap(),
                            content: vec![
                                OscPacket::Message(OscMessage {
                                    addr: "/bundle_info".to_string(),
                                    args: vec![OscType::String("real_time_packet".to_string())],
                                }),
                                OscPacket::Message(OscMessage {
                                    addr: "/info_msg".to_string(),
                                    args: vec![OscType::Time(
                                        OscTime::try_from(tick_time).unwrap(),
                                    )],
                                }),
                                pct.clone(),
                            ],
                        })
                    } else {
                        pct.clone()
                    }
                });

                for packet in send_packets {
                    if let OscPacket::Bundle(o) = packet.clone() {
                        let sys: SystemTime = o.timetag.into();
                        let datetime: DateTime<Utc> = sys.into();
                        info!("MY MAN SENDTIME {}", datetime.format("%d/%m/%Y %T"));
                    }

                    osc_client.send(packet);
                }
            }
        },
    );

    let addr = config::get_addr(cfg.application_in_port);
    info!("STARTING OSC READER");

    OSCStack::init(addr)
        .on_message("/set_bpm", &|msg| {
            let args = msg.clone().args;
            let arg = args.get(0).clone();
            match arg {
                None => {
                    warn!("Unable to parse set_bpm message (missing arg)")
                }
                Some(val) => match val.clone().int() {
                    None => {
                        warn!("set_bpm arg not an int")
                    }
                    Some(contained_val) => {
                        info!("SET BPM");
                        osc_pub_mutex
                            .lock()
                            .unwrap()
                            .try_push(LocalSequencerMessage::SetBpm(contained_val))
                            .unwrap();
                    }
                },
            }
        })
        .on_message("/reset_all", &|_msg| {
            info!("RESET ALL");
            osc_pub_mutex
                .lock()
                .unwrap()
                .try_push(LocalSequencerMessage::Reset)
                .unwrap();
        })
        .on_message("/hard_stop", &|_msg| {
            info!("HARD STOP");
            osc_pub_mutex
                .lock()
                .unwrap()
                .try_push(LocalSequencerMessage::HardStop)
                .unwrap();
        })
        .on_message("/wipe_on_finish", &|_msg| {
            info!("WIPE ON FINISH");
            osc_pub_mutex
                .lock()
                .unwrap()
                .try_push(LocalSequencerMessage::EndAfterFinish)
                .unwrap();
        })
        .on_tbundle("batch_update_queues", &|tbundle| {
            match BatchUpdateQueuesMessage::from_bundle(tbundle) {
                Ok(batch_update_msg) => {
                    if batch_update_msg.stop_missing {
                        info!("END AFTER FINISH");
                        osc_pub_mutex
                            .lock()
                            .unwrap()
                            .try_push(LocalSequencerMessage::EndAfterFinish)
                            .unwrap();
                    }

                    for update_queue_msg in batch_update_msg.update_queue_messages {
                        let alias = update_queue_msg.alias.clone();

                        let payload = sequencing_daemon::to_sequence(update_queue_msg.messages);

                        info!("Updating queue for {}", &alias);

                        let payload_local = LocalSequencerMessage::Queue(LocalQueuePayload {
                            sequencer_alias: alias,
                            entries: payload.message_sequence,
                            end_beat: payload.end_beat,
                            one_shot: update_queue_msg.one_shot,
                        });

                        info!("QUEUE CHANGED");
                        osc_pub_mutex
                            .lock()
                            .unwrap()
                            .try_push(payload_local)
                            .unwrap();
                    }
                }
                Err(e) => {
                    warn!("Failed to parse batch update queue message: {}", e);
                }
            }
        })
        .on_tbundle(
            "update_queue",
            &|tbundle| match UpdateQueueMessage::from_bundle(tbundle) {
                Ok(update_queue_msg) => {
                    let alias = update_queue_msg.alias.clone();

                    let payload = sequencing_daemon::to_sequence(update_queue_msg.messages);

                    info!("Updating queue for {}", &alias);

                    let payload_local = LocalSequencerMessage::Queue(LocalQueuePayload {
                        sequencer_alias: alias,
                        entries: payload.message_sequence,
                        end_beat: payload.end_beat,
                        one_shot: update_queue_msg.one_shot,
                    });

                    osc_pub_mutex
                        .lock()
                        .unwrap()
                        .try_push(payload_local)
                        .unwrap();
                }
                Err(e) => {
                    warn!("Failed to parse update_queue message: {}", e);
                }
            },
        )
        .begin();
}
