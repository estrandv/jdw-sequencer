package com.jackdaw.jdwsequencer

import com.jackdaw.jdwsequencer.model.RestInputNote
import com.jackdaw.jdwsequencer.model.SequencerNote
import com.jackdaw.jdwsequencer.model.beatsToMilliSeconds
import java.time.LocalDateTime
import java.time.temporal.ChronoField
import java.util.*

/*
    Holds two sets of notes: the CURRENT and the QUEUE.
    Each time getNext() is called, we look for the next note in CURRENT.
    If one or more is found, we return them and remove them from CURRENT.
    If CURRENT is empty when getNext() is called, we de-queue QUEUE into CURRENT and
        the loop starts over.
    The same QUEUE will be used over and over until replaced.
 */
class SequencePlayer {

    /*
        Due to how the looping of notes works we need somewhere to save the length of the
        last note in a current set. That way, the next queued set can start after the
        reserved time of the last set finishes.
     */
    private var lastCurrentSetEndNoteTime = 0.0

    private var currentNotes: MutableList<SequencerNote> = Collections.emptyList()
    private var queuedNotes: MutableList<RestInputNote> = Collections.emptyList()

    private var loopStartTime: LocalDateTime = LocalDateTime.now()

    fun queue(notes: List<RestInputNote>) {
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

            val start = beatsToMilliSeconds(it.startBeat, bpm)
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