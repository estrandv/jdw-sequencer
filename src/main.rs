#![feature(result_flattening, proc_macro_hygiene, decl_macro)]

use std::cell::RefCell;

use std::convert::TryFrom;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use config::REAL_TIME_MODE;
use jdw_osc_lib::model::TaggedBundle;
use local_messaging::{LocalQueuePayload, LocalSequencerMessage};
use log::{info, warn};
use master_sequencer::MasterSequencer;
use ringbuf::traits::{Producer, Split};
use ringbuf::HeapRb;
use rosc::{OscBundle, OscMessage, OscPacket, OscTime, OscType};
use sequencing_daemon::SequencingDaemonState;
use simple_logger::SimpleLogger;

use bundle_model::UpdateQueueMessage;

use crate::bundle_model::BatchUpdateQueuesMessage;
use crate::osc_communication::OSCClient;
use jdw_osc_lib::osc_stack::OSCStack;

mod bundle_model;
mod config;
mod local_messaging;
mod master_sequencer;
pub mod midi_utils;
mod osc_communication;
mod sequencer;
mod sequencing_daemon;

/*

    main.rs starts the two central loops:
        1. The osc polling loop, that handles input and writes it to the shared state.
        2. The sequencing loop, which reads from input and progresses the sequencers with the passage of time.

*/

fn main() {
    // Handles all log macros, e.g. "warn!()" to print info in terminal
    SimpleLogger::new()
        .with_level(config::LOG_LEVEL)
        .init()
        .unwrap();

    // NOTE: Expecting quite a few messages might arrive at the same time, account for this in sequencer loop
    let osc_pipe = HeapRb::<LocalSequencerMessage<OscPacket>>::new(100);
    let (mut osc_pub, mut osc_sub) = osc_pipe.split();

    // Note: Bit of a mess, but speed is not important for publishing here
    let osc_pub_mutex = Arc::new(Mutex::new(osc_pub));

    let osc_client = OSCClient::new();

    let master = MasterSequencer::new(
        master_sequencer::SequencerStartMode::WithLongestSequence,
        master_sequencer::SequencerResetMode::Individual,
    );

    // Start sequencer loop in separate thread and handle ticked packets
    sequencing_daemon::start_live_loop::<OscPacket, _>(
        master,
        120,
        osc_sub,
        move |packets_to_send, tick_time| {
            if !packets_to_send.is_empty() {
                // TODO: tick_time is actually a DateTime<Utc>, we create this system time alongside it for easier OscTime conversion.
                // There will always be a possible gap until this is streamlined in both ends
                info!("TICK! {:?}", tick_time);

                let send_packets = packets_to_send.iter().map(|pct| {
                    if REAL_TIME_MODE {
                        // Note: A regular bundle would have contained a time tag by default,
                        // but in order to provide proper context data we're sending a full tagged bundle.
                        // TODO: Depending on how you interpret the tagged bundle, you can probably just grab the timetag anyway
                        // and skip the info_msg (duplicate) time contents entirely.
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

                // TODO: This will always be a timing bottleneck when there are many packages
                // Ideally, you send them all in a bundle, but something will eventually have to unpack it and send it on for router subscriptions filtering to work.
                // One might consider buffered sending for more even volumes (e.g. 5 messages per loop), but if that is another
                // app in itself there is a translation cost ...
                // It is however easy to place a simple buffer right here. Might be worth trying to even out the massive drop on 0.0,
                //  although the eventual delay for buffered notes might be a bit too noticible and randomly distributed...
                for packet in send_packets {
                    if let OscPacket::Bundle(o) = packet.clone() {
                        let sys: SystemTime = o.timetag.into();
                        let datetime: DateTime<Utc> = sys.into();
                        println!("MY MAN SENDTIME {}", datetime.format("%d/%m/%Y %T"));
                    }

                    osc_client.send(packet);
                }
            }
        },
    );

    let addr = config::get_addr(config::APPLICATION_IN_PORT);
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
                        // Same as a call to "wipe on finish" - the order is then immediately reversed
                        //  for mentioned tracks when a new queue() is called
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
