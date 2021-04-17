use zmq;
use std::sync::{Arc, Mutex};
use crate::model::{QueueMetaData, SequencerNoteMessage, SequencerQueueData};
use crate::StateHandle;
use zmq::Socket;
use std::thread;
use serde_json;
use std::collections::HashMap;
use std::cell::RefCell;

// Open two connections
// Start a loop that polls the subscriber end
// Out connection (REQ) has to be available as ARC because main thread will call it
// THUS: Make a subscriber that has access to poller and everything it needs to access (ARCS)
// We can just put the REQ one in the main thread, it's not needed here

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

            if msg_type == "JDW.SEQ.QUE.NOTES" {
                let payload: Vec<SequencerNoteMessage> = serde_json::from_str(&json_msg).unwrap_or(Vec::new());

                if payload.is_empty() {
                    println!("WARN: Received empty or malformed JDW.SEQ.QUE.NOTES");
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

                        // TODO: Function is a bit bloated here, the queue mutation could easily be its own func
                        println!("Queueing: {:?} to {}", value.clone(), alias);

                        // Clear any pre-existing queue data of that alias
                        queue_data.lock().unwrap().queue.borrow_mut().retain(|e| *e.id != alias);

                        // Create a new queue entry for the alias containing all the notes in the request
                        queue_data.lock().unwrap().queue.borrow_mut().push(SequencerQueueData {
                            id: alias,
                            target_type: crate::model::OutputTargetType::Prosc,
                            instrument_id: value.get(0).unwrap().clone().target, // TODO: instrument id will not be needed here in the future
                            queue: RefCell::new(value)
                        });

                        // Notify the main thread that queue has been updated
                        queue_data.lock().unwrap().updated.replace(true);
                    }
                }
            } // else ... (other messages)

        }

    });
}