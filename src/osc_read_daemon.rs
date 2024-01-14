use crate::osc_client::{OSCClient, OSCPoller};
use rosc::{OscBundle, OscMessage, OscPacket, OscTime, OscType};
use jdw_osc_lib::TaggedBundle;


/*
    Handler for all incoming osc traffic. 
    Rewrite of earlier inline in main loop.
    Should ideally have hooks/closures for all specific messages and such instead of directly referencing 
        a specific state or sequencer. 
*/

/*

    TODO: Redesign plan. 
    - Should have two methods: onMessage and onTaggedBundle
    - Each of these should update a map of <string,closure> with a provided operation
        - As in: We keep a map to dictate what happens on specific messages or bundles 
    - This should allow OSCRead to assign things like:
        - onMessage("/set_bpm", |OscMessage| {...})
        - onTaggedBundle("update_queue", |TaggedBundle| {...})
    - This does not necessarily save an awful amount of code, but it is reusable and 
        neat and keeps it tidy in the main loop. 

    TODO: Alternative, more primitive redesign plan 
    - Simple return of either a tagged bundle or message, with which you can do as you like from the caller
    - Much like OscPacket, a trait with match could be implemented here

    - Scan: Like today, but should return a result of ProcessedOsc, Error 
        - Inline the casting to bundle and error message wrapping 
        - Then do any other voodoo outside of this struct 

*/

// Like the OscPacket enum, but with the internal type "TaggedBundle" replacing "OscBundle"
pub enum ProcessedOsc {
    Message(OscMessage),
    Bundle(TaggedBundle)
}

struct OSCRead {
    poller: OSCPoller,
}

impl OSCRead {
    fn scan(&mut self) -> Result<ProcessedOsc, String> {
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
