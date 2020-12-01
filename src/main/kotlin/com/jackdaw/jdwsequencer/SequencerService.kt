package com.jackdaw.jdwsequencer

import com.jackdaw.jdwsequencer.model.msToBeats
import org.springframework.stereotype.Component
import kotlinx.coroutines.*
import java.time.LocalDateTime
import java.time.Period
import java.time.temporal.ChronoUnit

/*
    Manages the main sequencing loop, calling all other forms of sequencers and syncers
 */
@Component
class SequencerService(
        val proscPlayerManager: ProscPlayerManager
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

            if (beatCounter >= 1.0) {
                // TODO: Call sync all each beat. Possibly 4 times per beat.
                beatCounter = 0.0
            }

            proscPlayerManager.playNext(LocalDateTime.now(), bpm)
            lastTick = now
            delay(tickMillis)
        }
    }

}