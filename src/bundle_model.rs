use rosc::{OscBundle, OscMessage, OscPacket};

use jdw_osc_lib::{OscArgHandler, TaggedBundle, TimedOSCPacket};


/*

    Collection of parsing models for received TaggedBundles.

*/


/*
    Tagged bundle example: 
    [info: update_queue]
    [update_queue_info: "my_alias"]
    bundle: [timed_msg_bundle, timed_msg_bundle ...]
             
*/
pub struct UpdateQueueMessage {
    pub alias: String,
    pub messages: Vec<TimedOSCPacket>,
}

impl UpdateQueueMessage {
    pub fn from_bundle(bundle: TaggedBundle) -> Result<UpdateQueueMessage, String> {
        if &bundle.bundle_tag != "update_queue" {
            return Err(format!("Attempted to parse {} as update_queue bundle", &bundle.bundle_tag));
        }
        
        let info_msg = bundle.get_message(0)?;
        info_msg.expect_addr("/update_queue_info")?;
        let alias = info_msg.get_string_at(0, "alias")?;

        let msg_bundle = bundle.get_bundle(1)?;
        let mut contained_timed_messages: Vec<TimedOSCPacket> = Vec::new();
        for packet in msg_bundle.content {
            match packet {
                OscPacket::Bundle(bun) => {
                    let tagged_bun = TaggedBundle::new(&bun)?;
                    let timed_message = TimedOSCPacket::from_bundle(tagged_bun)?;
                    contained_timed_messages.push(timed_message);
                },
                _ => println!("Found a non-bundle in the update queue"),
            }
        }

        Ok(UpdateQueueMessage {
            alias,
            messages: contained_timed_messages
        })
       
    }
}