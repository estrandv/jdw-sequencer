use std::{cell::RefCell, str::FromStr, sync::Arc, thread, time::SystemTime};

use bigdecimal::BigDecimal;
use jdw_osc_lib::model::TimedOSCPacket;
use log::{debug, info, warn};
use ringbuf::{storage::Heap, traits::Consumer, wrap::caching::Caching, SharedRb};
use rosc::OscPacket;

use crate::{
    local_messaging::LocalSequencerMessage, master_sequencer::MasterSequencer, midi_utils,
    sequencer::SequencerEntry,
};

/*
    Below struct and function are a rewrite of logic previously contained in queue.rs::shift_queue.
    They should live somewhere else, determined by usage.
*/

pub struct OscSequencePayload {
    pub message_sequence: Vec<SequencerEntry<OscPacket>>,
    pub end_beat: BigDecimal,
}

pub fn to_sequence(input: Vec<TimedOSCPacket>) -> OscSequencePayload {
    let mut new_sequence: Vec<SequencerEntry<OscPacket>> = vec![];
    let mut new_timeline = BigDecimal::from_str("0.0").unwrap();

    for packet in &input {
        new_sequence.push(SequencerEntry::new(
            new_timeline.clone(),
            packet.packet.clone(),
        ));

        new_timeline += packet.time.clone();
    }

    // TODO: Note the composite payload - sequencer.rs takes an end_beat for queue
    OscSequencePayload {
        message_sequence: new_sequence,
        end_beat: new_timeline,
    }
}

pub struct SequencingDaemonState {
    pub bpm: RefCell<i32>,
    pub reset: RefCell<bool>,
    pub hard_stop: RefCell<bool>,
}

impl SequencingDaemonState {
    pub fn new(bpm_param: i32) -> SequencingDaemonState {
        SequencingDaemonState {
            bpm: RefCell::new(bpm_param),
            reset: RefCell::new(false),
            hard_stop: RefCell::new(false),
        }
    }
}

pub fn start_live_loop<T: 'static + Clone + Send, F>(
    mut master_sequencer: MasterSequencer<T>,
    bpm_param: i32,
    mut message_sub: Caching<Arc<SharedRb<Heap<LocalSequencerMessage<T>>>>, false, true>,
    entry_operations: F,
) where
    F: 'static + Send + Fn(Vec<T>, SystemTime) -> (),
{
    thread::spawn(move || {
        let state = SequencingDaemonState::new(bpm_param);

        let mut last_loop_time: Option<SystemTime> = None;

        let sleeper = spin_sleep::SpinSleeper::new(100);

        loop {
            let tick_time_sys = SystemTime::now();
            let elapsed_ns: u64 = match last_loop_time {
                Some(t) => tick_time_sys
                    .duration_since(t)
                    .unwrap_or_default()
                    .as_nanos() as u64,
                None => 0,
            };
            last_loop_time = Some(tick_time_sys);

            let current_bpm = state.bpm.clone();
            let reset_requested = state.reset.clone().into_inner();
            let hard_stop_requested = state.hard_stop.clone().into_inner();
            {
                state.reset.replace(false);
                state.hard_stop.replace(false);
            }

            let elapsed_beats =
                midi_utils::duration_to_beats(elapsed_ns, current_bpm.clone().into_inner());

            if hard_stop_requested {
                master_sequencer.force_wipe();
            } else {
                master_sequencer.start_check();

                if reset_requested {
                    master_sequencer.force_reset();
                } else {
                    master_sequencer.reset_check();
                }
                let collected = master_sequencer.tick(elapsed_beats);

                entry_operations(collected, tick_time_sys);
            }

            while let Some(msg) = message_sub.try_pop() {
                debug!("POP");

                match msg {
                    LocalSequencerMessage::HardStop => {
                        state.hard_stop.replace(true);
                    }
                    LocalSequencerMessage::Reset => {
                        state.reset.replace(true);
                    }
                    LocalSequencerMessage::SetBpm(new_bpm) => {
                        state.bpm.replace(new_bpm);
                    }
                    LocalSequencerMessage::EndAfterFinish => {
                        master_sequencer.end_after_finish();
                    }
                    LocalSequencerMessage::Queue(payload) => {
                        info!("QUEUE RECEIVED");
                        master_sequencer.queue(
                            payload.sequencer_alias.as_str(),
                            payload.entries,
                            payload.end_beat,
                            payload.one_shot,
                        );
                    }
                    LocalSequencerMessage::BatchQueue(payloads) => {
                        for payload in payloads {
                            master_sequencer.queue(
                                payload.sequencer_alias.as_str(),
                                payload.entries,
                                payload.end_beat,
                                payload.one_shot,
                            );
                        }
                    }
                }
            }

            let now = SystemTime::now();
            let time_taken_ns = now
                .duration_since(tick_time_sys)
                .unwrap_or_default()
                .as_nanos() as u64;

            let tick_time_ns = crate::config::Config::get().tick_time_us * 1000;

            if time_taken_ns > tick_time_ns {
                warn!(
                    "Operations performed (time: {}) exceed tick time, overflow...",
                    time_taken_ns
                );
            }
            debug!("End loop");
            let time_left_until_tick = if time_taken_ns < tick_time_ns {
                tick_time_ns - time_taken_ns
            } else {
                0
            };
            sleeper.sleep(std::time::Duration::from_nanos(time_left_until_tick));
        }
    });
}
