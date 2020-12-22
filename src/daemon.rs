use crate::player_management::{PROSCPlayerManager};
use std::sync::{Arc, Mutex};
use std::cell::{RefCell, Cell};
use chrono::{DateTime, Utc};
use std::thread;
use std::thread::sleep;
use std::time;


pub struct SequencerDaemon {
    prosc_player_manager: Arc<Mutex<PROSCPlayerManager>>,
    bpm: Arc<Mutex<i32>>,
    tick_interval_ms: u64,
    beat_counter: Arc<Mutex<Cell<f32>>>,
    last_tick_time: Arc<Mutex<Cell<DateTime<Utc>>>>
}

impl SequencerDaemon {
    pub fn new(
        ppm: Arc<Mutex<PROSCPlayerManager>>
    ) -> SequencerDaemon {
        SequencerDaemon {
            prosc_player_manager: ppm,
            bpm: Arc::new(Mutex::new(120)), // TODO: Replacable cell value
            tick_interval_ms: 2,
            beat_counter: Arc::new(Mutex::new(Cell::new(0.0))),
            last_tick_time: Arc::new(Mutex::new(Cell::new(chrono::offset::Utc::now()))),
        }
    }

    pub fn start(this: Arc<Mutex<SequencerDaemon>>) {

        thread::spawn(move || {
            loop {
                let now = chrono::offset::Utc::now();
                let elapsed = now.time() - this.lock().unwrap().last_tick_time.lock().unwrap()
                    .get()
                    .time();
                let ms_elapsed = crate::model::midi_utils::ms_to_beats(
                    elapsed.num_milliseconds(),
                    this.lock().unwrap().bpm.lock().unwrap().clone()
                ) ;

                {
                    let bpm = this.lock().unwrap()
                        .bpm.lock().unwrap().clone();

                    this.lock().unwrap()
                        .prosc_player_manager.lock().unwrap()
                        .play_next(
                            now,
                                bpm
                        );
                }

                // TODO: Midi sync according to beat
                this.lock().unwrap().beat_counter.lock().unwrap().update(| v| v + ms_elapsed);
                this.lock().unwrap().last_tick_time.lock().unwrap().replace(now);
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