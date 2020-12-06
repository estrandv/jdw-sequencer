use crate::player_management::{PROSCPlayerManager};
use std::sync::{Arc, Mutex};
use std::cell::{RefCell, Cell};
use chrono::{DateTime, Utc};
use std::thread;
use std::thread::sleep;
use std::time;


struct SequencerDaemon {
    prosc_player_manager: Arc<Mutex<PROSCPlayerManager>>,
    bpm: Arc<Mutex<i32>>,
    tick_interval_ms: u64,
    beat_counter: Arc<Mutex<Cell<f32>>>,
    last_tick_time: Arc<Mutex<DateTime<Utc>>>
}

impl SequencerDaemon {
   pub fn start(this: Arc<Mutex<SequencerDaemon>>) {
        thread::spawn(move || {
            loop {
                let now = chrono::offset::Utc::now();
                let elapsed = now.time() - this.lock().unwrap().last_tick_time.lock().unwrap().time();
                let ms_elapsed = crate::model::midi_utils::ms_to_beats(
                    elapsed.num_milliseconds(),
                    this.lock().unwrap().bpm.lock().unwrap().clone()
                ) ;
                this.lock().unwrap().beat_counter.lock().unwrap().update(|mut v| v + ms_elapsed);
                sleep(time::Duration::from_millis(this.lock().unwrap().tick_interval_ms));

            }
        });
   }
}

/*
class SequencerService(
        val prosc_player_manager: ProscPlayerManager,
        val restClient: RestClient
) {

    var bpm = 60
    private val tickMillis = 10L

    private var lastTick: LocalDateTime = LocalDateTime.now()
    private var beatCounter: Double = 0.0

    fun start() = runBlocking {

        while (true) {
            val now = LocalDateTime.now()
            val timeElapsed = ChronoUnit.MILLIS.between(lastTick, now)
            beatCounter += msToBeats(timeElapsed, bpm)

            // Sync 24 times per beat/half-note according to MIDI protocol standards
            // TODO: In the far future we should have a separate "jdw-midi-sync-service"
            //  that sends sync to both this application and midi. More reusable, better
            //  separation of concern.
            if (beatCounter >= 1.0 / 24.0) {
                restClient.midiSync()
                beatCounter = 0.0
            }

            prosc_player_manager.playNext(LocalDateTime.now(), bpm)
            lastTick = now
            delay(tickMillis)
        }
    }

}
 */