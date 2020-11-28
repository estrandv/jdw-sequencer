package com.jackdaw.jdwsequencer.model

import java.time.LocalDateTime
import java.time.temporal.ChronoField
import java.time.temporal.TemporalAmount
import java.util.*
import kotlin.collections.ArrayList

class SequencePlayer {

    private var currentNotes: MutableList <PlayerNote> = Collections.emptyList();
    private var queuedNotes: MutableList<PlayerNote> = Collections.emptyList();

    private var loopStartTime: LocalDateTime = LocalDateTime.now();

    // TODO: Might need tweaking for seconds/milliseconds but otherwise sound. Should be in UTIL.
    fun beatsToMilliSeconds(beat: Double, bpm: Int): Long {
        return (beat * (60.0 / bpm)).toLong();
    }

    fun queue(notes: List<Note>) {
        queuedNotes = mutableListOf();

        // TODO: Relativetime and sysclock are different formats; need BPM calc to know time comparison

        var beat = 0.0
        for (note in notes) {
            queuedNotes.add(PlayerNote(
                    note.tone,
                    note.amplitude,
                    note.sustain_time,
                    beat
            ))
            beat += note.reserved_time
        }
    }

    fun reset(atTime: LocalDateTime) {
        currentNotes = mutableListOf()
        currentNotes.addAll(queuedNotes)
        loopStartTime = atTime
    }

    fun getNext(atTime: LocalDateTime, bpm: Int): List<PlayerNote> {
        currentNotes.ifEmpty { reset(atTime) }

        return currentNotes.filter {
            loopStartTime.plus(
                    beatsToMilliSeconds(it.startBeat, bpm),
                    ChronoField.MILLI_OF_DAY.baseUnit
            ).isBefore(atTime)
        }
    }

    /*
        TODO: Trying to figure out this whole "tick" thing.

        I feel that ideally it should go something like this:
        1. Since all notes for all players share the same timeline, we need
            to combine all players in order to arrange them in order. The alternative
            is to run some obscene "get next for all and compare" every time we tick a note.
        2. To add clock syncs into that we'd have to add those the same as notes to the mix, perhaps
            changing into some kind of "midiPayload" instead of "note".
        3. Old sequencer had a slight desync problem where the recalculation at each loop reset
            put everything slightly off beat. There is really no way to sync this without involving
            the system clock somehow; we need to compare with the actual time rather than just sleep
            relative. THus "ticks" where we increment time by very small units and compare sysclock
            to note times on each "tick". This in turn requires that we freeze a "startTime" when the
            loop launches.
        4. The massive "allnotesplayer" needs to provide a "getNext" that can enter the queue seamlessly.
            The lifecycle of this player would be something like:
            a. The loop is started. The player ticks but doesn't play anything since nothing is queued
                or current.
            b. A payload comes in detailing a set of notes to queue for a set of different outputs.
            c. The notes are shuffled into playerNote objects that also contain the output name and then
                sorted together in a large list. This list is placed as queue in the big player.
            d. On each tick, the player calls getNotesAt(currentTime). The player has a startTime that
            is set each time the queue resets. When notes are played in current set, they are removed.
            getNotesAt always fetches from the current set. If the current set is empty, the queued set
            is copied into current and the startTime reset to current systime.
                - Here's a dimentionality problem: WE need to do this at a lower level as well.
                Each output needs to have its own player and they need to handle this themselves.
                So we CAN'T group everything together. We need to call getAllPlayerNotesAt() which
                collects all notes from all players local getNotesAt() method. This way a longer
                player can continue it's set while a shorter one resets.
         5. To summarize this class:
            a. Needs a queue set, a current set and a start time
            b. Notes must be assigned non-relative values by adding their sustains/reserves together
            c. These values will be "time after start". With getNotesOnAt() we fetch a list of all notes
            with a timeAfterStart that matches current systime.
            d. We also have a getNotesOffAt(). This is really tricky since sustain should be able to
                continue after the loop resets.
            e. reset() sets start time to systime, copies queue to current and then continues ticking as normal
                - When the last noteOn set is handled we need some kind of transition queue to kick in.
                This queue will contain what remains of the current notes prior to reset.
                YOU ARE HERE. THIS IS THE IDEA. 

     */
}