package com.jackdaw.jdwsequencer

import com.jackdaw.jdwsequencer.model.RestInputNote
import org.springframework.stereotype.Component
import java.time.LocalDateTime

/*
    Keeps track of underlying map of sequence players, sending their notes to
    the prosc (supercollider) endpoint with playNext()
 */
@Component
class ProscPlayerManager(
        val restClient: RestClient
) {

    // <Output Name, Player>
    private var proscPlayers: MutableMap<String, SequencePlayer> = mutableMapOf();

    fun playNext(time: LocalDateTime, bpm: Int) {

        for (player in proscPlayers) {
            val notesOnTime = player.value.getNext(time, bpm)
            if (notesOnTime.size > 1) {
                println("WARNING: Same output ${player.key} playing multiple notes" +
                        " at once. Your tickrate might be too slow for the given BPM/NoteLength.")
            }
            restClient.postProsc(player.key, notesOnTime)
        }
    }

    fun queue(outputName: String, notes: List<RestInputNote>) {
        if (!proscPlayers.containsKey(outputName)) {
            proscPlayers[outputName] = SequencePlayer()
        }

        proscPlayers[outputName]?.queue(notes)

    }

}