use crate::model::SequencerNote;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SNewMessage {
    synth: String,
    values: Vec<OSCValueField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OSCValueField {
    name: String,
    value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct  MIDIMessage {
    tone: f32,
    sustain_time: f32, 
    amplitude: f32,
}


// TODO: Contact midi server
pub fn sync_midi() -> Result<(), reqwest::Error> {
    Ok(())
}

impl SequencerNote {
    pub fn convert(&self, output_key: &str) -> SNewMessage {
        SNewMessage {
            synth: output_key.to_string(),
            values: vec!(
                OSCValueField{name: "amp".to_string(), value: self.amplitude},
                OSCValueField{name: "sus".to_string(), value: self.sustain},
                OSCValueField{name: "freq".to_string(), value: self.tone as f32},
            )
        }
    }
}


// #[instrument] // Enables extra logging for things that can go wrong in-call.
pub fn post_prosc_samples(notes: Vec<SNewMessage>) -> Result<(), reqwest::Error> {


    /*
        Current PROSC sample setup doesn't use much fancy note stuff.
        What we do is basically save the name and tone and dump everything else.
        Tone has to be int because it's expected to be the plain number of the sample
            for the named sample pack.
        If this sounds hacky it's because it is; sampling should probably not use NOTE at
            all and instead queue some kind of sample_data object. But that will take some
            major refactoring...
     */
    let url = format!("http://localhost:5000/sample/");

    let json = serde_json::json!(notes);

    //println!("Posting to {}, Message: {}", url.clone(), &json);

    reqwest::blocking::Client::new().post(&url)
        .json(&json)
        .send()?;

    Ok(())

}

// #[instrument] // Enables extra logging for things that can go wrong in-call.
pub fn post_prosc_notes(notes: Vec<SNewMessage>) -> Result<(), reqwest::Error> {

    let url = "http://localhost:5000/impl/s_new";

    let json = serde_json::json!(notes);

    //println!("Posting to {}, Message: {}", &url, &json);

    reqwest::blocking::Client::new().post(url)
        .json(&json)
        .send()?;

    Ok(())

}

/*
    Separate note posting implementation for calling the jdw-midi-server api.
    Clumsily duplicated due to poor understanding of rust traits and lifetimes.
 */
//#[instrument] // Enables extra logging for things that can go wrong in-call.
pub fn post_midi_notes(output_key: &str, notes: Vec<SequencerNote>) -> Result<(), reqwest::Error> {

    let url = format!("http://localhost:11000/play/{}", output_key);

    for note in notes {

        let message = MIDIMessage{
            tone: note.tone,
            sustain_time: note.sustain,
            amplitude: note.amplitude
        };

        let json = serde_json::json!(message);

        reqwest::blocking::Client::new().post(&url)
            .json(&json)
            .send()?;
    }

    Ok(())

}

