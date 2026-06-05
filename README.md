# JackDAW OSC Sequencer

A beat-synchronous sequencer engine for the JackDAW system. Accepts timed OSC packet bundles registered under named aliases and loops through them continuously.

## How It Works

Clients send bundles of `[beat, osc_packet]` pairs for a given alias, forming a sequence. The sequencer loops through each sequence at the configured BPM, dispatching the OSC packets at their scheduled beats. Multiple named sequences run simultaneously, orchestrated by a `MasterSequencer`.

## Features

- **Multiple simultaneous sequencers** — each alias runs independently
- **Flexible start modes** — nearest, longest-sequence-first, or immediate
- **Reset modes** — all-after-longest or individual sequence reset
- **One-shot sequences** — fire-once, don't loop
- **Batch updates** — replace entire queue state atomically
- **MIDI sync** — sync infrastructure in place (currently disabled)
- **Real-time bundles** — packets wrapped in timing bundles for the OSC router

## Architecture

The sequencer runs two threads:
1. **OSC poll loop** — receives sequence data and control commands on a UDP port
2. **Sequencing loop** — ticks at 5ms resolution, dispatches scheduled packets

## Protocol

Send sequence data as tagged OSC bundles to the sequencer port. Each bundle contains:
- A `/bundle_info` header
- Packet pairs: `[beat (float), osc_packet (bundle)]`

## Dependencies

- `rosc` — OSC encoding/decoding
- `jdw-osc-lib` — shared OSC protocol library
- `chrono`, `bigdecimal`, `spin_sleep` — timing
- `ringbuf` — lock-free ring buffer for inter-thread communication
