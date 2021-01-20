use crate::player_management::{PlayerManager};
use std::sync::{Arc, Mutex};
use std::cell::{RefCell, Cell};
use chrono::{DateTime, Utc};
use std::thread;
use std::thread::sleep;
use std::time;
use crate::rest::RestClient;


pub struct SequencerDaemon {
    prosc_player_manager: Arc<Mutex<PlayerManager>>,
    rest_client: Arc<Mutex<RestClient>>,
    pub bpm: Arc<Mutex<Cell<i32>>>,
    tick_interval_ms: u64,
    beat_counter: Arc<Mutex<Cell<f32>>>,
    last_tick_time: Arc<Mutex<Cell<DateTime<Utc>>>>,
    pub silenced: Arc<Mutex<Cell<bool>>>
}

impl SequencerDaemon {
    pub fn new(
        ppm: Arc<Mutex<PlayerManager>>,
        rc: Arc<Mutex<RestClient>>
    ) -> SequencerDaemon {
        SequencerDaemon {
            prosc_player_manager: ppm,
            rest_client: rc,
            bpm: Arc::new(Mutex::new(Cell::new(120))),
            tick_interval_ms: 2,
            beat_counter: Arc::new(Mutex::new(Cell::new(0.0))),
            last_tick_time: Arc::new(Mutex::new(Cell::new(chrono::offset::Utc::now()))),
            silenced: Arc::new(Mutex::new(Cell::new(false)))
        }
    }

    pub fn bpm(&self, set_to: i32) {
        self.bpm.lock().unwrap().replace(set_to);
    }
    pub fn silenced(&self, set_to: bool) {
        self.silenced.lock().unwrap().replace(set_to);
    }

    pub fn start(this: Arc<Mutex<SequencerDaemon>>) {

        thread::spawn(move || {
            loop {
                let now = chrono::offset::Utc::now();
                let elapsed = now.time() - this.lock().unwrap().last_tick_time.lock().unwrap()
                    .get()
                    .time();

                let beats_elapsed = crate::model::midi_utils::ms_to_beats(
                    elapsed.num_milliseconds(),
                    this.lock().unwrap().bpm.lock().unwrap().get().clone()
                ) ;

                {
                    // Only order note playing if not silenced
                    let slc = this.lock().unwrap().silenced.lock().unwrap().get();
                    if !slc {
                        let bpm = this.lock().unwrap()
                            .bpm.lock().unwrap().clone();

                        this.lock().unwrap()
                            .prosc_player_manager.lock().unwrap()
                            .play_next(
                                now,
                                bpm.get()
                            );
                    }
                }

                this.lock().unwrap().beat_counter.lock().unwrap().update(| v| v + beats_elapsed);

                // Send sync every 1/24 beat as specified by midi protocol
                if this.lock().unwrap().beat_counter.lock().unwrap().get() >= 1.0 / 24.0 {
                    this.lock().unwrap().rest_client.lock().unwrap().sync_midi();
                    this.lock().unwrap().beat_counter.lock().unwrap().set(0.0);
                }

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