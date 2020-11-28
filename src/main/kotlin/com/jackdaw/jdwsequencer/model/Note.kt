package com.jackdaw.jdwsequencer.model

data class Note(
        val tone: Double,
        val reserved_time: Double,
        val sustain_time: Double,
        val amplitude: Double
)