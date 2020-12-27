use crate::model::sequencer::SequencerNote;
use log::info;
use crate::model::prosc_api::{SNewMessage, OSCValueField};
use crate::model::midi_api::MIDIMessage;

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
    pub fn post_prosc_samples(&self, output_key: &str, notes: Vec<SequencerNote>) -> Result<(), reqwest::Error> {

        for note in notes {

            /*
                Current PROSC sample setup doesn't use much fancy note stuff.
                What we do is basically save the name and tone and dump everything else.
                Tone has to be int because it's expected to be the plain number of the sample
                    for the named sample pack.
                If this sounds hacky it's because it is; sampling should probably not use NOTE at
                    all and instead queue some kind of sample_data object. But that will take some
                    major refactoring...
             */
            let url = format!("http://localhost:5000/sample/{}/{}", output_key, note.tone as i32);
            // Not also how we're using SNewMessage even though basically none of the
            // provided data is used. This is mostly left here for later added features and
            // is pretty much useless atm.
            let message = SNewMessage::new(
                output_key.to_string(),
                vec!(
                    OSCValueField::new("amp", note.amplitude),
                    )
            );

            let json = serde_json::json!(message);

            println!("Posting to {}, Message: {}", url.clone(), &json);

            self.r_client.post(&url)
                .json(&json)
                .send()?;
        }

        Ok(())

    }

    #[instrument] // Enables extra logging for things that can go wrong in-call.
    pub fn post_prosc_notes(&self, output_key: &str, notes: Vec<SequencerNote>) -> Result<(), reqwest::Error> {

        let url = "http://localhost:5000/impl/s_new";

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

    /*
        Separate note posting implementation for calling the jdw-midi-server api.
        Clumsily duplicated due to poor understanding of rust traits and lifetimes.
     */
    #[instrument] // Enables extra logging for things that can go wrong in-call.
    pub fn post_midi_notes(&self, output_key: &str, notes: Vec<SequencerNote>) -> Result<(), reqwest::Error> {

        let url = format!("http://localhost:11000/play/{}", output_key);

        for note in notes {

            let message = MIDIMessage::new(
                note.tone,
                note.sustain,
                note.amplitude
            );

            let json = serde_json::json!(message);

            self.r_client.post(&url)
                .json(&json)
                .send()?;
        }

        Ok(())

    }

}