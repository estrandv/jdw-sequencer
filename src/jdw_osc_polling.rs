use crate::osc_communication::{OSCClient, OSCPoller};
use rosc::{OscBundle, OscMessage, OscPacket, OscTime, OscType};
use jdw_osc_lib::TaggedBundle;

/*

    Simplifies OSC polling by returning either a Message or (in-house type) TaggedBundle. 

*/


// Like the OscPacket enum, but with the internal type "TaggedBundle" replacing "OscBundle"
pub enum ProcessedOsc {
    Message(OscMessage),
    Bundle(TaggedBundle)
}

pub struct JDWOSCPoller {
    poller: OSCPoller,
}

impl JDWOSCPoller {

    pub fn new() -> JDWOSCPoller {
        JDWOSCPoller { poller: OSCPoller::new() }
    }

    pub fn scan(&mut self) -> Result<ProcessedOsc, String> {
        return match self.poller.poll() {
            Ok(osc_packet) => {
                return match osc_packet {
                    OscPacket::Message(osc_msg) => Ok(ProcessedOsc::Message(osc_msg)),
                    OscPacket::Bundle(osc_bundle) => {

                        return match TaggedBundle::new(&osc_bundle) {
                            Ok(tagged_bundle) => Ok(ProcessedOsc::Bundle(tagged_bundle)),
                            Err(msg) => Err(format!("Failed to parse update_queue message: {}", msg))
                        };
                    }
                };
            }
            Err(error_msg) => Err(error_msg)
        };
    }

}
