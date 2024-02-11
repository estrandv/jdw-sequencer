use rosc::OscPacket;

use jdw_osc_lib::model::{OscArgHandler, TaggedBundle, TimedOSCPacket};


/*

    Collection of parsing models for received TaggedBundles.

*/

// Just a collection of the below, useful for wiping non-mentioned queues 
// TODO: Usage: (1) receive, (2) mark all non-contained aliases as ending on next loop, (3) make sure newly queued aliases also remove that flag if there 
// NOTE: Might also be viable, if batches are very large, to simply send "stop queue" messages for everything and then requeue
/*
    Tagged bundle example: 
    [info: batch_update_queue]
    [batch_update_queue_info: 1]
    bundle: [update_queue_bundle, update_queue_bundle, ...]
             
*/
pub struct BatchUpdateQueuesMessage {
    pub update_queue_messages: Vec<UpdateQueueMessage>,
    pub stop_missing: bool
}

impl BatchUpdateQueuesMessage {
    pub fn from_bundle(bundle: TaggedBundle) -> Result<BatchUpdateQueuesMessage, String> {
        if &bundle.bundle_tag != "batch_update_queue" {
            return Err(format!("Attempted to parse {} as update_queue bundle", &bundle.bundle_tag));
        }

        let info_msg = bundle.get_message(0)?;
        info_msg.expect_addr("/batch_update_queue_info")?;
        let stop_missing_int = info_msg.get_int_at(0, "stop_missing")?;

        let msg_bundle = bundle.get_bundle(1)?;
        let mut queue_updates: Vec<UpdateQueueMessage> = Vec::new();
        for packet in msg_bundle.content {
            match packet {
                OscPacket::Bundle(bun) => {
                    let tagged_bun = TaggedBundle::new(&bun)?;
                    let queue_update = UpdateQueueMessage::from_bundle(tagged_bun)?;
                    queue_updates.push(queue_update);
                },
                _ => println!("Found a non-bundle in the update queue"),
            }
        }

        Ok(BatchUpdateQueuesMessage {
            update_queue_messages: queue_updates,
            stop_missing: if stop_missing_int == 1 {true} else {false}
        })


    } 
}

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