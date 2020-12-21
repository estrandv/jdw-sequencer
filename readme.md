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
    sequencer is stopped or a new set is queued. Note that the sequencer waits
    for the longest note set to finish before resetting all players, so this setup:
    
    player1: c c c c
    player2: f 
    
... would not play f 4 times in player2; it would play it once and then wait for
    player1 to finish before playing again. 
    
### Usage example
If PROSC is running and contains a syndef named "blipp", this example post call
    would queue two notes for repeated playing:
    
    curl -H 'Content-Type: application/json' \
    --data '[{"tone": 155.56349186104046, "sustain_time": 0.550000011920929, "reserved_time": 0.5, "amplitude": 0.6399999856948853}, {"tone": 155.56349186104046, "sustain_time": 0.5, "reserved_time": 0.5, "amplitude": 0.5600000023841858}]' \
    http://localhost:8000/queue/prosc/blipp/blipp1
    
Note that the last past of the url is the alias (see api.rs). You can queue 
    many different simultaneous sets to the same output (e.g. "/blipp/")
    if you assign different aliases to your queue calls. Calling queue on the 
    same alias will replace the last queued set.