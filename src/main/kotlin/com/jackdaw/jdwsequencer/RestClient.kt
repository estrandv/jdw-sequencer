package com.jackdaw.jdwsequencer

import com.fasterxml.jackson.databind.ObjectMapper
import com.github.kittinunf.fuel.Fuel
import com.github.kittinunf.fuel.core.extensions.jsonBody
import com.jackdaw.jdwsequencer.model.SequencerNote
// https://stackoverflow.com/questions/65043370/type-mismatch-when-serializing-data-class
import org.springframework.stereotype.Component

// TODO: All outgoing calls should be to a config-specified endpoint
@Component
class RestClient {

    fun postProsc(outputName: String, notes: List<SequencerNote>) {

        if (notes.isNotEmpty()) {
            val mapper = ObjectMapper()
            val root = mapper.createObjectNode()
            root.putArray("args").add(outputName)
                    .add(-1)
                    .add(0)
                    .add(0)
                    .add("freq")
                    .add(notes[0].tone)
                    .add("amp")
                    .add(notes[0].amplitude)
                    .add("sus")
                    .add(notes[0].sustain)

            // PROSC
            Fuel.post("http://localhost:5000/osc/s_new")
                    .jsonBody(mapper.writeValueAsString(root))
                    .also { println(it) }
                    .response { _ ->  }
        }
    }

    fun midiSync() {
        // jdw-midi-server TODO: Doesn't seem to do anything
        Fuel.post("http://localhost:8000/sync_all")
    }

}