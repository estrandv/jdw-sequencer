use rosc::{OscBundle, OscMessage, OscPacket};

// TODO: Stand-in file until all osc handling is ported to a common library

/*
    Adding some convenience functions for OscMessage args
 */
trait OscArgHandler {
    fn expect_addr(&self, addr_name: &str) -> Result<(), String>;
    fn expect_args(&self, amount: usize) -> Result<String, String>;
    fn get_string_at(&self, index: usize, name: &str, ) -> Result<String, String>;
    fn get_float_at(&self, index: usize, name: &str, ) -> Result<f32, String>;
    fn get_int_at(&self, index: usize, name: &str, ) -> Result<i32, String>;
}

impl OscArgHandler for OscMessage {

    fn expect_addr(&self, addr_name: &str) -> Result<(), String> {
        if &self.addr.to_string() != addr_name {
            return Err(format!("Attempted to format {} as the wrong kind of message - this likely a human error in the source code", addr_name));
        }

        Ok(())
    }

    fn expect_args(&self, amount: usize) -> Result<String, String> {

        if self.args.len() < amount {
            return Err(format!("Message did not contain the {} first required args.", amount));
        }

        Ok("Ok".to_string())
    }

    fn get_string_at(&self, index: usize, name: &str, ) -> Result<String, String> {
        let err_msg = format!("{} string not found as {}th arg", name, index);
        self.args
            .get(index)
            .map_or(None, |some| some.clone().string())
            .map_or(Err(err_msg), |s| Ok(s))
    }

    fn get_float_at(&self, index: usize, name: &str, ) -> Result<f32, String> {
        let err_msg = format!("{} float not found as {}th arg", name, index);
        self.args
            .get(index)
            .map_or(None, |some| some.clone().float())
            .map_or(Err(err_msg), |s| Ok(s))
    }

    fn get_int_at(&self, index: usize, name: &str, ) -> Result<i32, String> {
        let err_msg = format!("{} float not found as {}th arg", name, index);
        self.args
            .get(index)
            .map_or(None, |some| some.clone().int())
            .map_or(Err(err_msg), |s| Ok(s))
    }

}


/*
    In order to properly utilize bundles I have created a standard where the first
        packet in every JDW-compatible bundle is an OSC message with a bundle type
        string contained within, e.g.: ["/bundle_tag", "nrt_record_request"]
 */
 #[derive(Debug)]
pub struct TaggedBundle {
    pub bundle_tag: String,
    pub contents: Vec<OscPacket>
}

impl TaggedBundle {
    pub fn new(bundle: &OscBundle) -> Result<TaggedBundle, String> {
        let first_msg = match bundle.content.get(0).ok_or("Empty bundle")?.clone() {
            OscPacket::Message(msg) => { Option::Some(msg) }
            OscPacket::Bundle(_) => {Option::None}
        }.ok_or("First element in bundle not an info message!")?;

        if first_msg.addr != "/bundle_info" {
            return Err(format!("Expected /bundle_info as first message in bundle, got: {}", &first_msg.addr));
        }

        let bundle_tag = first_msg.args.get(0)
            .ok_or("bundle info empty")?
            .clone()
            .string().ok_or("bundle info should be a string")?;

        let contents = if bundle.content.len() > 1 {(&bundle.content[1..].to_vec()).clone()} else {vec![]};

        Ok(TaggedBundle {
            bundle_tag,
            contents
        })
    }

    fn get_message(&self, content_index: usize) -> Result<OscMessage, String> {
        self.contents.get(content_index)
            .map(|pct| pct.clone())
            .ok_or("Invalid index".to_string())
            .map(|pct| match pct {
                OscPacket::Message(msg) => {
                    Ok(msg)
                }
                _ => {Err("Not a message".to_string())}
            })
            .flatten()
    }

    fn get_bundle(&self, content_index: usize) -> Result<OscBundle, String> {
        self.contents.get(content_index)
            .map(|pct| pct.clone())
            .ok_or("Invalid index".to_string())
            .map(|pct| match pct {
                OscPacket::Bundle(msg) => {
                    Ok(msg)
                }
                _ => {Err("Not a bundle".to_string())}
            })
            .flatten()
    }
}

/*
    Timed osc messages are used to delay execution. This has uses both for NRT recording as
        well as sequencer spacing or timed gate off messages.
    [/bundle_info, "timed_msg"]
    [/timed_msg_info, 0.0]
    [... msg ...]
 */
#[derive(Debug, Clone)]
pub struct TimedOscMessage {
    pub time: f32,
    pub message: OscMessage
}

impl TimedOscMessage {
    pub fn from_bundle(bundle: TaggedBundle) -> Result<TimedOscMessage, String>{
        if &bundle.bundle_tag != "timed_msg" {
            return Err(format!("Attempted to parse {} as timed_msg bundle", &bundle.bundle_tag));
        }

        let info_msg = bundle.get_message(0)?;
        let actual_msg = bundle.get_message(1)?;

        info_msg.expect_addr("/timed_msg_info")?;
        let time = info_msg.get_float_at(0, "time")?;

        Ok(TimedOscMessage {
            time,
            message: actual_msg
        })

    }
}

// TODO: Non-standard implementations below

/*
    [info: update_queue]
    [update_queue_info: "my_alias"]
    bundle: [timed_msg_bundle, timed_msg_budle ...]
             
*/
pub struct UpdateQueueMessage {
    pub alias: String,
    pub messages: Vec<TimedOscMessage>,
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
        let mut contained_timed_messages: Vec<TimedOscMessage> = Vec::new();
        for packet in msg_bundle.content {
            match packet {
                OscPacket::Bundle(bun) => {
                    let tagged_bun = TaggedBundle::new(&bun)?;
                    let timed_message = TimedOscMessage::from_bundle(tagged_bun)?;
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