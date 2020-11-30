package com.jackdaw.jdwsequencer.model

import kotlinx.serialization.Serializable

@Serializable
data class SequencerNote(
        val tone: Double,
        val amplitude: Double,
        val sustain: Double,
        val startBeat: Double
)