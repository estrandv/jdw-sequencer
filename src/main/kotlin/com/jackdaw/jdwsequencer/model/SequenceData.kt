package com.jackdaw.jdwsequencer.model

import kotlinx.serialization.Serializable

@Serializable
data class SequenceData(
        val notes: List<InputNote>,
        val output_name: String
)