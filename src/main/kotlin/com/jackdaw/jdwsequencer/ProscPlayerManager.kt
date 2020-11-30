package com.jackdaw.jdwsequencer

import com.jackdaw.jdwsequencer.model.InputNote
import com.jackdaw.jdwsequencer.model.SequencePlayer
import org.springframework.stereotype.Component
import java.time.LocalDateTime

// TODO: Even if prosc/midi can be split, there still needs
//  to be a global bpm manager to handle sync sends for midi
@Component
class ProscPlayerManager(
        val restClient: RestClient
) {

    private var proscPlayers: MutableMap<String, SequencePlayer> = mutableMapOf();

    fun playNext(time: LocalDateTime, bpm: Int) {

        println("Playing next...")
        for (player in proscPlayers) {
            // TODO: Warn if next contains more than one note (overflow)

            restClient.postProsc(player.key, player.value.getNext(time, bpm))
        }
    }

    fun queue(outputName: String, notes: List<InputNote>) {
        if (!proscPlayers.containsKey(outputName)) {
            proscPlayers[outputName] = SequencePlayer()
        }

        proscPlayers[outputName]?.queue(notes)

        println("DEBUG: Reached the wired candidate!")

    }

}