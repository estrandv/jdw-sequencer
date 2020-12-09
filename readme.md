# Jackdaw Sequencer
- Inputs sets of notes via rest 
- Loops and outputs note data to other rest services for playing
- Interacts with PROSC for sending supercollider tone plays: https://bitbucket.org/Emstrevk/jdw-prosc
- Interacts with jdw-midi-server for sending MIDI sync and tones: https://bitbucket.org/Emstrevk/jdw-midi-server
- See API.rs for endpoints

### How it works
The sequencer starts a "clock" loop on launch, which will tick at a hardcoded interval.
    A list of "sequence players" are maintained, which can contain a set of notes.
    On each tick, the sequencer will ask the players to play any notes whose start time
    matches the current system time.
    
Notes are added to players via rest calls to the sequencer. A sequencer of notes with 
    relative start times are queued and this set will loop infinitely until the 
    sequencer is stopped or a new set is queued.