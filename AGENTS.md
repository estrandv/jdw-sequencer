# AGENTS.md — jdw-sequencer

## Source Structure

```
src/
  main.rs                 # Entry: OSC poll thread + sequencing thread
  master_sequencer.rs     # MasterSequencer — manages all named sequences
  sequencer.rs            # Individual Sequencer — loop + dispatch logic
  sequencing_daemon.rs    # High-precision timing loop
  bundle_model.rs         # UpdateQueueMessage, BatchUpdateQueuesMessage
  config.rs               # Port and tick-rate configuration
  osc_communication.rs    # OSC send helper (to router port 13339)
  local_messaging.rs      # Ring-buffer channels for thread communication
```

## Key Types

- `MasterSequencer` — orchestrates all aliases. Methods: `register()`, `unregister()`, `start_mode()`, `reset_mode()`, `receive_batch_update()`
- `Sequencer` — per-alias. Fields: `alias`, `bpm`, `loop_enabled`, `sequence: Vec<TimedOSCPacket>`
- `TimedOSCPacket` — from jdw-osc-lib: `{ beat: BigDecimal, packet: OscPacket }`
- `StartMode` — `Nearest`, `LongestSequence`, `Immediate`
- `ResetMode` — `AllAfterLongest`, `Individual`

## Thread Model

```
┌─────────────────┐     ringbuf     ┌──────────────────┐
│  OSC poll loop  │ ──────────────> │  Sequencing loop │
│  (UDP receive)  │                 │  (5ms tick,      │
│                 │                 │   dispatches)     │
└─────────────────┘                 └──────────────────┘
```

## Configuration

- Default port: configured in `config.rs`
- Tick rate: 5ms (configurable)
- BPM: received per-sequence via OSC
- Output: sends to OSC router at `127.0.0.1:13339`

## Build & Run

```bash
cargo build --release
cargo run --release
```

## Common Modifications

- To add a new start mode: add variant to `StartMode` enum in `master_sequencer.rs`, implement in `MasterSequencer::start()`
- To change tick resolution: update `TICK_MS` in `config.rs`
- To add a new control message: add variant to the internal message enum, handle in the sequencing loop
