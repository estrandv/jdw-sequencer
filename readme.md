# TODO: REwrite whole readme once ZeroMQ migration is complete

# Jackdaw Sequencer
- Inputs sets of notes via ZeroMQ (See jdw-broker) 
- Loops and outputs timed messages to ZeroMQ based on the message <time> attribtue
- Interacts with PROSC for sending supercollider tone plays: https://bitbucket.org/Emstrevk/jdw-prosc
- Interacts with jdw-midi-server for sending MIDI sync and tones: https://bitbucket.org/Emstrevk/jdw-midi-server
- See zeromq.rs 

### How it works
The sequencer starts a "clock" loop on launch, which will tick at a hardcoded interval.
    A list of "sequence players" are maintained, which can contain a set of notes.
    On each tick, the sequencer will ask the players to play any notes whose start time
    matches the current system time.
    
Messages (typically "notes") are sent to the sequencer as a queue call. A sequencer of messages with 
    relative start times are queued and this set will loop infinitely until the 
    sequencer is stopped or a new set is queued. Note that the sequencer waits
    for the longest message set to finish before resetting all players, so this setup:
    
    player1: c c c c
    player2: f 
    
... would not play f 4 times in player2; it would play it once and then wait for
    player1 to finish before playing again. 
   

### TODO: Usage Example
- For now, refer to jdw-pycompose for example calls 
