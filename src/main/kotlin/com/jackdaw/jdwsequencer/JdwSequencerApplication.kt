package com.jackdaw.jdwsequencer

import org.springframework.boot.autoconfigure.SpringBootApplication
import org.springframework.boot.runApplication

@SpringBootApplication
class JdwSequencerApplication

fun main(args: Array<String>) {
	runApplication<JdwSequencerApplication>(*args)
}
