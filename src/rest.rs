use crate::model::sequencer::SequencerNote;
use log::info;
use crate::model::prosc_api::{SNewMessage, OSCValueField};

use tracing::instrument;

#[derive(Debug)]
pub struct RestClient{
    r_client: reqwest::blocking::Client
}

impl RestClient {

    pub fn new() -> RestClient {
        RestClient {
            r_client: reqwest::blocking::Client::new()
        }
    }

    #[instrument] // Enables extra logging for things that can go wrong in-call.
    pub fn post_notes(&self, url: &str, output_key: &str, notes: Vec<SequencerNote>) -> Result<(), reqwest::Error> {

        for note in notes {

            let message = SNewMessage::new(
                output_key.to_string(),
                vec!(
                    OSCValueField::new("amp", note.amplitude),
                    OSCValueField::new("sus", note.sustain),
                    OSCValueField::new("freq", note.tone as f32),
                )
            );

            let json = serde_json::json!(message);

            println!("Posting to {}, Message: {}", &url, &json);

            self.r_client.post(url)
                .json(&json)
                .send()?;
        }

        Ok(())

    }

}