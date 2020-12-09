use crate::model::sequencer::SequencerNote;
use log::info;
use crate::model::prosc_api::{SNewMessage, OSCValueField};

use tracing::instrument;

#[derive(Debug)]
pub struct RestClient{
    r_client: reqwest::blocking::Client,
    url: String
}

impl RestClient {

    pub fn new() -> RestClient {
        RestClient {
            r_client: reqwest::blocking::Client::new(),
            url: "http://localhost:5000/".to_string()
        }
    }

    #[instrument] // Enables extra logging for things that can go wrong in-call.
    pub fn local_post_prosc(&self, output_key: &str, notes: Vec<SequencerNote>) -> Result<(), reqwest::Error> {

        let url = format!("{}{}", self.url, "impl/s_new");

        // TODO: it's just the one note right now
        let note = notes.get(0).unwrap();

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

        self.r_client.post(&url)
            .json(&json)
            .send()?;

        //info!("Response: {:?}", response);

        Ok(())

    }

}