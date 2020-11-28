package com.jackdaw.jdwsequencer

import com.jackdaw.jdwsequencer.model.SequenceData
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.PostMapping
import org.springframework.web.bind.annotation.RequestBody
import org.springframework.web.bind.annotation.RestController

@RestController
class RestController(
        @Autowired val proscPlayerManager: ProscPlayerManager
) {

    @PostMapping(
            path = ["/queue"],
            consumes = ["application/json"],
            produces = ["application/json"]
    )
    fun queue(@RequestBody data: SequenceData) {
        proscPlayerManager.queue(data.output_name, data.notes)
    }

    @GetMapping(path = ["/"])
    fun test() {
        proscPlayerManager.queue("hi", mutableListOf())
    }

}