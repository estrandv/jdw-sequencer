package com.jackdaw.jdwsequencer

import com.jackdaw.jdwsequencer.model.Note
import com.jackdaw.jdwsequencer.model.SequencePlayer
import org.springframework.stereotype.Component
import java.time.LocalDateTime

@Component
class ProscPlayerManager(
        val restClient: RestClient
) {

    private var proscPlayers: MutableMap<String, SequencePlayer> = mutableMapOf();

    fun playNext(time: LocalDateTime, bpm: Int) {
        for (player in proscPlayers) {
            // TODO: Warn if next contains more than one note (overflow)
            restClient.postProsc(player.key, player.value.getNext(time, bpm))
        }
    }

    fun queue(outputName: String, notes: List<Note>) {
        if (!proscPlayers.containsKey(outputName)) {
            proscPlayers[outputName] = SequencePlayer()
        }

        proscPlayers[outputName]?.queue(notes)

        println("DEBUG: Reached the wired candidate!")

    }

}