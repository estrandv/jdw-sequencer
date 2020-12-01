package com.jackdaw.jdwsequencer.model

import kotlinx.serialization.Serializable

@Serializable
data class RestInputNote(
        val tone: Double,
        val reserved_time: Double,
        val sustain_time: Double,
        val amplitude: Double
)