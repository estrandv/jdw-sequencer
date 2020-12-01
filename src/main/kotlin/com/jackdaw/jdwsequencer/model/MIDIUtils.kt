package com.jackdaw.jdwsequencer.model

fun beatsToMilliSeconds(beat: Double, bpm: Int): Long {
    return (beat * (60.0 / bpm) * 1000).toLong();
}

// TODO: Reversed from function above. Anton suggested it might be Beat = (ms/1000) / (60/bpm)
fun msToBeats(ms: Long, bpm: Int): Double {
    return ((ms.toDouble()/1000) * 60) / bpm
}