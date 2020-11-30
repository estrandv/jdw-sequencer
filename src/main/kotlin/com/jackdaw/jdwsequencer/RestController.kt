package com.jackdaw.jdwsequencer

import com.jackdaw.jdwsequencer.model.InputNote
import com.jackdaw.jdwsequencer.model.SequenceData
import com.jackdaw.jdwsequencer.model.SequencerNote
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.PostMapping
import org.springframework.web.bind.annotation.RequestBody
import org.springframework.web.bind.annotation.RestController

@RestController
class RestController(
        val proscPlayerManager: ProscPlayerManager,
        val sequencerService: SequencerService,
        val restClient: RestClient
) {

    @PostMapping(
            path = ["/queue"],
            consumes = ["application/json"],
            produces = ["application/json"]
    )
    fun queue(@RequestBody data: SequenceData) {
        proscPlayerManager.queue(data.output_name, data.notes)
    }

    @GetMapping(path = ["/testQueue"])
    fun testQueue() {
        proscPlayerManager.queue(
                "blipp",
                listOf(
                    InputNote(440.0, 0.5, 0.4, 1.0),
                    InputNote(1240.0, 1.0, 0.4, 0.8),
                    InputNote(650.0, 0.25, 0.3, 1.0),
                    InputNote(650.0, 0.25, 1.0, 1.0),
        ))
        sequencerService.start()
    }

    @GetMapping(path = ["/testNote"])
    fun test() {

        val note = SequencerNote(
                440.0,
                1.0,
                2.0,
                0.0
        )

        restClient.postProsc("blipp", arrayListOf(note))

    }

}