use crate::model::sequencer::SequencerNote;
use reqwest::Client;
use log::info;

pub struct RestClient{
    r_client: Client,
    url: String
}

impl RestClient {

    pub fn new() -> RestClient {
        RestClient {
            r_client: Client::new(),
            url: "http://localhost:5000/".to_string()
        }
    }

    pub async fn post_prosc(&self, output_key: &str, notes: Vec<SequencerNote>) -> Result<(), reqwest::Error> {
        let url = format!("{}{}", self.url, "osc/s_new");

        let response = self.r_client.post(&url)
            .json(&serde_json::json!(notes))
            .send()
            .await?
            .json()
            .await?;

        info!("Response: {:?}", response);

        Ok(())

    }

}