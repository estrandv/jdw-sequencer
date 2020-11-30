package com.jackdaw.jdwsequencer

import org.springframework.stereotype.Component
import kotlinx.coroutines.*
import java.time.LocalDateTime

@Component
class SequencerService(
        val proscPlayerManager: ProscPlayerManager
) {

    var bpm = 60
    val tickMillis = 10L

    fun start() = runBlocking {

        while (true) {
            println("How many milliseconds?")
            proscPlayerManager.playNext(LocalDateTime.now(), bpm)
            delay(tickMillis)
        }
    }

}