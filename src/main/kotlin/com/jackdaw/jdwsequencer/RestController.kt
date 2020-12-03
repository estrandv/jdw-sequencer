package com.jackdaw.jdwsequencer

import com.jackdaw.jdwsequencer.model.RestInputNote
import com.jackdaw.jdwsequencer.model.RestInputSequenceData
import org.springframework.web.bind.annotation.*
import org.springframework.web.bind.annotation.RestController

// TODO: Port/url should be configurable and example calls outlined
@RestController
class RestController(
        val proscPlayerManager: ProscPlayerManager,
        val sequencerService: SequencerService,
        val restClient: RestClient
) {

    @PostMapping(
            path = ["/queue/{output}"],
            consumes = ["application/json"],
            produces = ["application/json"]
    )
    fun queue(@RequestBody data: RestInputSequenceData, @PathVariable output: String) {
        proscPlayerManager.queue(output, data.notes)
    }

    @GetMapping(path = ["/bpm/{bpm}"])
    fun bpm(@PathVariable bpm: Int) {
        sequencerService.bpm = bpm
    }

    @GetMapping(path = ["/testQueue/{output}"])
    fun testQueue(@PathVariable output: String) {
        proscPlayerManager.queue(
                output,
                listOf(
                    RestInputNote(240.0, 1.5, 4.4, 1.0),
                    RestInputNote(1240.0, 1.0, 0.4, 0.8),
                    RestInputNote(650.0, 0.25, 1.3, 0.4),
                    RestInputNote(850.0, 0.25, 1.0, 1.0),
        ))
        sequencerService.start()
    }

}