package com.jackdaw.jdwsequencer.model

import java.time.LocalDateTime
import java.time.temporal.ChronoField
import java.util.*

class SequencePlayer {

    /*
        Due to how the looping of notes works we need somewhere to save the length of the
        last note in a current set. That way, the next queued set can start after the
        reserved time of the last set finishes.
     */
    private var lastCurrentSetEndNoteTime = 0.0

    private var currentNotes: MutableList<SequencerNote> = Collections.emptyList()
    private var queuedNotes: MutableList<InputNote> = Collections.emptyList()

    private var loopStartTime: LocalDateTime = LocalDateTime.now()

    // TODO: Might need tweaking for seconds/milliseconds but otherwise sound. Should be in UTIL.
    fun beatsToMilliSeconds(beat: Double, bpm: Int): Long {
        return (beat * (60.0 / bpm) * 1000).toLong();
    }

    fun queue(notes: List<InputNote>) {
        queuedNotes = mutableListOf()
        queuedNotes.addAll(notes)
    }

    private fun shuffleQueue() {

        currentNotes = mutableListOf()
        if (queuedNotes.isNotEmpty()) {

            var beat = lastCurrentSetEndNoteTime
            for (note in queuedNotes) {
                val new = SequencerNote(
                        note.tone,
                        note.amplitude,
                        note.sustain_time,
                        beat
                )
                currentNotes.add(new)

                beat += note.reserved_time

            }

            // Next time we load queued notes, use the last reserved time of
            // this set to create a starting point (so that the last note gets
            // time to finish)
            lastCurrentSetEndNoteTime = queuedNotes[queuedNotes.size -1].reserved_time

        }

    }

    fun getNext(atTime: LocalDateTime, bpm: Int): List<SequencerNote> {

        // Shuffle the queue into current
        currentNotes.ifEmpty {
            shuffleQueue()
            loopStartTime = atTime
        }

        println("At time $atTime")

        val candidates = currentNotes.filter {

            println("## For note: ${it.tone}")

            println("loopStartTime: $loopStartTime")

            println("Note startBeat: ${it.startBeat}")
            val start = beatsToMilliSeconds(it.startBeat, bpm)
            println("Note start relative: $start")
            val noteTime = loopStartTime.plus(
                    start,
                    ChronoField.MILLI_OF_SECOND.baseUnit
            )

            noteTime.isBefore(atTime)
        }

        currentNotes.removeAll(candidates)
        return candidates
    }
}