use zmq;
use std::sync::{Arc, Mutex};
use crate::queue::{ApplicationQueue, UnprocessedSequence, SequencerTickMessage};
use crate::{StateHandle};
use zmq::Socket;
use std::thread;
use std::collections::HashMap;
use std::cell::RefCell;
use serde::Serialize;
use serde_json;
use chrono::{DateTime, Utc};
use crate::model::{LoopStartMessage, SequencerWipeMessage};

pub struct PublishingClient {
    socket: Socket
}

impl PublishingClient {
    pub fn new() -> Self {
        let context = zmq::Context::new();
        let socket = context.socket(zmq::PUSH).unwrap();
        socket.connect("tcp://localhost:5559").unwrap();
        //socket.set_req_relaxed(true); // Don't wait for replies from server on REQ
        PublishingClient {socket}
    }

    // TODO: Fix MIDI posting
    // MIDI needs to convert sustain time to actual seconds using BPM
    // Best way to convey that info is probably by supplying SEQ.START to MIDI with the
    // given BPM of the sequencer. Either that or by supplying the calculated time in the message itself
    // from pycompose.
    pub fn post_note(&self, notes: Vec<SequencerTickMessage>, timestamp: DateTime<Utc>) {

        // NOTE: Keepign commented for compiling example
        let unwrapped: Vec<_> = notes.iter().map(|n| n.msg.clone()).collect();
        //let payload = format!("JDW.SEQ.BATCH::{}::{}", timestamp.to_rfc3339(), serde_json::to_string(&unwrapped).unwrap_or("[]".to_string()));
        //self.socket.send(&payload, 0);
    }

    pub fn post_midi_sync(&self) {
        self.socket.send("JDW.MIDI.SYNC::".as_bytes(), 0);
        self.socket.recv_string(0);
    }

    pub fn post_loop_start(&self, time: DateTime<Utc>, bpm: i32) {
        let msg = LoopStartMessage {
            time: time.to_rfc3339(),
            bpm
        };

        self.socket.send(format!("JDW.SEQ.BEGIN::{}", serde_json::to_string(&msg).unwrap()).as_bytes(), 0);
        self.socket.recv_string(0);
    }
}

pub fn poll(
    queue_data: Arc<Mutex<ApplicationQueue>>,
    state_handle: Arc<Mutex<StateHandle>>,
    bpm: Arc<Mutex<RefCell<i32>>>,
) {

    thread::spawn(move || {
        let context = zmq::Context::new();
        let socket = context.socket(zmq::SUB).unwrap();
        socket.connect("tcp://localhost:5560").unwrap();
        socket.set_subscribe("JDW.SEQ.BPM".as_bytes());
        socket.set_subscribe("JDW.SEQ.QUEUE".as_bytes());
        socket.set_subscribe("JDW.SEQ.WIPE".as_bytes());

        loop {
            let msg = socket.recv_msg(0).unwrap();

            let decoded_msg = msg.as_str().unwrap().split("::").collect::<Vec<&str>>();

            let msg_type = decoded_msg.get(0).unwrap().to_string();
            let msg_timestamp = decoded_msg.get(1).unwrap_or(&"").to_string();
            let type_handle = format!("{}::{}::", msg_type, msg_timestamp);

            println!("message: {}", &type_handle);

            let json_msg = msg.as_str()
                .unwrap()
                .split(&type_handle)
                .collect::<Vec<&str>>()
                .get(1).unwrap_or(&"").to_string();

            //println!("nested: {}", json_msg.clone());

            if msg_type == String::from("JDW.SEQ.QUEUE") {
                //let payload: Vec<SequencerTickMessage> = serde_json::from_str(&json_msg).unwrap_or(Vec::new());
                let payload = vec![]; // Note: commented for compiling example

                if payload.is_empty() {
                    println!("WARN: Received empty or malformed JDW.SEQ.QUEUE payload");
                }

                queue_data.lock().unwrap().update_queue(payload);
            } else if msg_type == String::from("JDW.SEQ.WIPE") {
                let payload: Vec<SequencerWipeMessage> = serde_json::from_str(&json_msg).unwrap_or(Vec::new());
                let aliases: Vec<String>= payload.iter().map(|p| p.alias.to_string()).collect();
                queue_data.lock().unwrap().queue.borrow_mut().retain(|e| !aliases.contains(&e.id));

                for alias in aliases {
                    queue_data.lock().unwrap().queue.borrow_mut().push(UnprocessedSequence {
                        id: alias,
                        queue: RefCell::new(Vec::new())
                    });
                }

            } else if msg_type == String::from("JDW.SEQ.BPM") {
                bpm.lock().unwrap().replace(serde_json::from_str(&json_msg).unwrap());
            } else {
                panic!("Unknown message type: {}", msg_type);
            }

            // TODO: BPM only appears to come through when the application shuts down or starts
            //  It is possible that some kind of async solution is required
            //  Also confused about why messages only seem to pass through the router when unblocked
            //  the BPM message does not exist until sequencer shuts down

        }

    });
}

