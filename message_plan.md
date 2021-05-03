# Phase out the typed sequencer note messages

### Why?
1. Sequencer should not care what the messages do; it repeats sequences, that is all.
2. If there are no demands on format, the sequencer can forward literally anything without 
    extra handling (which in itself is just pointless boilerplate code anyway).
3. Currently, the sequencer uses the AMP value to determine if a note should be played or 
    is just a silence. This is an "architecture smell" since a sequencer that didn't care
    about the message contents could just receive a blank message to signify amp 0 notes anyway.

### How?
1. main.rs "decoded_msg" performs the first split. The new format should be a JSON like so:
   BLA.BLA.SEQ::{"res": 1.3, "alias": "bla", "message": "BLA.BLA.NOTE::{...}"} where the actual note stuff 
   is just a plain string. We should thus create a {res, alias, message} object which we pass to the queue.
   Split between output type should be removed.
2. main.rs update_queue parses a vector of "note messages". As such BLA.BLA.SEQ should probably 
    be of format: BLA.BLA.SEQ::[{...}], which I assume it currently already is. This of course 
    means that some attributes might have to change places to make more sense...
    SEQ::{"alias": "one", "payload": [{res, message}]}
3. Thus we need two objects:
    SequencerQueueMessage{alias: str, payload: Vec<>}
    SequenceMessage{res: f32, msg: str}
   OR we go with the same old "grouped by alias" and keep it as an array with everything by the same 
    alias: [{res, alias, message}]
4. Thus:
    - SequencerNoteMessage (the incoming message) chagned to the above definition 
    - SequencerQueueData should remove target_type and instrument id 
    - The rest should kinda follow (usages etc.)