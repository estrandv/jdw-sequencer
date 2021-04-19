use zmq;
use std::sync::{Arc, Mutex};
use crate::model::{QueueMetaData, SequencerNoteMessage, SequencerQueueData, OutputTargetType, MIDINotePlayMessage};
use crate::StateHandle;
use zmq::Socket;
use std::thread;
use std::collections::HashMap;
use std::cell::RefCell;
use serde::Serialize;
use serde_json;

pub struct PublishingClient {
    socket: Socket
}

impl PublishingClient {
    pub fn new() -> Self {
        let context = zmq::Context::new();
        let socket = context.socket(zmq::REQ).unwrap();
        socket.connect("tcp://localhost:5559").unwrap();
        socket.set_req_relaxed(true); // Don't wait for replies from server on REQ
        PublishingClient {socket}
    }

    pub fn post_note(&self, note: SequencerNoteMessage) {
        self.socket.send(format!("JDW.PLAY.NOTE::{}", serde_json::to_string(&note).unwrap()).as_bytes(), 0);
    }

    pub fn post_sample(&self, note: SequencerNoteMessage) {
        self.socket.send(format!("JDW.PLAY.SAMPLE::{}", serde_json::to_string(&note).unwrap()).as_bytes(), 0);
    }

    pub fn post_midi_note(&self, note: SequencerNoteMessage, bpm: i32) {
        let tone: f32 = match note.args.get("freq") {
            None => {
                println!("WARN: Supplied MIDI note had no <freq> arg, defaulted to 44");
                44.0
            }
            Some(value) => {*value}
        };

        let sus = match note.args.get("sus") {
            None => {
                println!("WARN: Supplied MIDI note had no <sus> arg, defaulted to 1");
                1.0
            }
            Some(value) => {*value}
        };

        let sus_ms = crate::midi_utils::beats_to_milli_seconds(sus, bpm);

        let amp = match note.args.get("amp") {
            None => {
                println!("WARN: Supplied MIDI note had no <amp> arg, defaulted to 1");
                1.0
            }
            Some(value) => {*value}
        };

        let midi_note = MIDINotePlayMessage {
            target: note.target,
            tone: tone as i32,
            sus_ms: sus_ms as f32,
            amp
        };

        self.socket.send(
            format!("JDW.MIDI.PLAY.NOTE::{}", serde_json::to_string(&midi_note).unwrap()).as_bytes(),
            0
        );

    }

    pub fn post_midi_sync(&self) {
        self.socket.send("JDW.MIDI.SYNC::".as_bytes(), 0);
    }
}


pub fn poll(queue_data: Arc<Mutex<QueueMetaData>>, state_handle: Arc<Mutex<StateHandle>>) {

    thread::spawn(move || {
        let context = zmq::Context::new();
        let socket = context.socket(zmq::SUB).unwrap();
        socket.connect("tcp://localhost:5560").unwrap();
        socket.set_subscribe("JDW.SEQ.".as_bytes());

        loop {
            let msg = socket.recv_msg(0).unwrap();
            let decoded_msg = msg.as_str().unwrap().split("::").collect::<Vec<&str>>();

            // JDW.SEQ.QUE.NOTES::[{"target": "blipp", "alias": "blipp1", "time": 0.0, "args": {"amp": 1.0}}]
            let msg_type = decoded_msg.get(0).unwrap().to_string();
            let json_msg = decoded_msg.get(1).unwrap_or(&"").to_string();

            if &msg_type == "JDW.SEQ.QUE.NOTES" {
                update_queue(json_msg, queue_data.clone(), OutputTargetType::Prosc)
            } else if &msg_type == "JDW.SEQ.QUE.SAMPLES" {
                update_queue(json_msg, queue_data.clone(), OutputTargetType::ProscSample)
            } else if &msg_type == "JDW.SEQ.QUE.MIDI" {
                update_queue(json_msg, queue_data.clone(), OutputTargetType::MIDI)
            }

        }

    });
}

fn update_queue(json_msg: String, queue_data: Arc<Mutex<QueueMetaData>>, posting_type: OutputTargetType) {
    let payload: Vec<SequencerNoteMessage> = serde_json::from_str(&json_msg).unwrap_or(Vec::new());

    if payload.is_empty() {
        println!("WARN: Received empty or malformed JDW.SEQ.QUE.*");
    }

    let mut grouped_by_alias: HashMap<String, Vec<SequencerNoteMessage>> = HashMap::new();
    for note in payload {
        if !grouped_by_alias.contains_key(&note.alias) {
            grouped_by_alias.insert(note.alias.to_string(), Vec::new());
        }

        grouped_by_alias.get_mut(&note.alias).unwrap().push(note);

    }

    println!("Parsed queue message: {:?}", &grouped_by_alias);

    for (alias, value) in grouped_by_alias {

        if !&value.is_empty() {

            println!("Queueing: {:?} to {}", value.clone(), alias);

            // Clear any pre-existing queue data of that alias
            queue_data.lock().unwrap().queue.borrow_mut().retain(|e| *e.id != alias);

            // Create a new queue entry for the alias containing all the notes in the request
            queue_data.lock().unwrap().queue.borrow_mut().push(SequencerQueueData {
                id: alias,
                target_type: posting_type.clone(),
                instrument_id: value.get(0).unwrap().clone().target, // TODO: instrument id will not be needed here in the future
                queue: RefCell::new(value)
            });

            // Notify the main thread that queue has been updated
            queue_data.lock().unwrap().updated.replace(true);
        }
    }
}