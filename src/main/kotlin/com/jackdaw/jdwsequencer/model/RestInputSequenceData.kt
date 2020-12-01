package com.jackdaw.jdwsequencer.model

import kotlinx.serialization.Serializable

@Serializable
data class RestInputSequenceData(
        val notes: List<RestInputNote>
)