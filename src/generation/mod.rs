mod bestiary;
mod place;

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

use anyhow::Result;
use crossbeam::channel::{Receiver, Sender, TryRecvError};
use futures::StreamExt;
use ollama_rs::{
    generation::{completion::request::GenerationRequest, options::GenerationOptions},
    Ollama,
};
use place::PlaceType;
use rand::{seq::IteratorRandom, SeedableRng};
use regex::Regex;
use serde::de::DeserializeOwned;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{
    config,
    mud::world::{Location, Place},
};

pub use place::{DUNGEON_PLACE_TYPE, VILLAGE_PLACE_TYPE};

#[derive(Debug)]
pub struct Generator {
    request_queue: UnboundedReceiver<GenerationReq>,
    response_queue: Sender<GenerationRes>,
    client: AIClient,
}

#[derive(Debug)]
pub struct GeneratorHandle {
    request_queue: UnboundedSender<GenerationReq>,
    response_queue: Receiver<GenerationRes>,
}

#[derive(Debug)]
pub enum GenerationReq {
    Places(PlaceType, usize),
}

#[derive(Debug)]
pub enum GenerationRes {
    Place(Place, HashMap<Location, Place>),
}

impl Generator {
    pub fn new() -> (Self, GeneratorHandle) {
        let (req_s, req_r) = tokio::sync::mpsc::unbounded_channel();
        let (res_s, res_r) = crossbeam::channel::unbounded();

        (
            Self {
                request_queue: req_r,
                response_queue: res_s,
                client: AIClient::new_random(config::get().tone_words.clone()),
            },
            GeneratorHandle {
                request_queue: req_s,
                response_queue: res_r,
            },
        )
    }

    pub async fn run(mut self) {
        loop {
            let req = self
                .request_queue
                .recv()
                .await
                .expect("Gen request channel shouldn't close");
            match req {
                GenerationReq::Places(place_type, count) => {
                    let client = self.client.clone();
                    let response_queue = self.response_queue.clone();

                    tokio::spawn(async move {
                        place::generate_places(&client, &place_type, count)
                            .await
                            .for_each_concurrent(3, |(place, places)| async {
                                response_queue
                                    .send(GenerationRes::Place(place, places))
                                    .expect("Gen response channel shouldn't close");
                            })
                            .await;
                    })
                }
            };
        }
    }
}

impl GeneratorHandle {
    pub fn request_generate(&mut self, req: GenerationReq) {
        self.request_queue
            .send(req)
            .expect("Gen handle request channel shouldn't close");
    }

    pub fn get_responses(&mut self) -> Option<GenerationRes> {
        match self.response_queue.try_recv() {
            Ok(r) => Some(r),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                unreachable!("Gen handle response channel shouldn't close")
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct AIClient {
    client: Ollama,
    seed: i32,
    tone: Vec<String>,
    /// We want to run deterministically for tests
    non_deterministic: bool,
}

impl AIClient {
    pub fn new_random(tone_words: Vec<String>) -> Self {
        AIClient {
            client: Ollama::default(),
            seed: rand::random(),
            tone: tone_words,
            non_deterministic: true,
        }
    }

    pub async fn generate_with_tone(&self, mut prompt: String) -> Result<String> {
        let hash: i32 = self.make_gen_hash(&prompt);

        let mut rng = rand::rngs::StdRng::seed_from_u64((self.seed ^ hash) as u64);
        let mut tone: String = "\nUse the following tone: ".into();
        let mut iter = self.tone.iter().map(|s| s.as_str());

        for i in 0..config::get().tone_words_per_generation {
            if let Some(s) = (&mut iter).choose(&mut rng) {
                tone.push_str(s);
                if i + 1 < config::get().tone_words_per_generation {
                    tone.push(',');
                    tone.push(' ');
                }
            }
        }

        prompt.push_str(&tone);

        self.generate(prompt, hash).await
    }

    pub async fn generate_simple(&self, prompt: String) -> Result<String> {
        let hash: i32 = self.make_gen_hash(&prompt);
        self.generate(prompt, hash).await
    }

    async fn generate(&self, prompt: String, hash: i32) -> Result<String> {
        Ok(self
            .client
            .generate(
                GenerationRequest::new("llama3:latest".to_string(), prompt).options(
                    GenerationOptions::default()
                        .seed(self.seed ^ hash)
                        .temperature(config::get().model_temperature),
                ),
            )
            .await?
            .response)
    }

    fn make_gen_hash(&self, prompt: &String) -> i32 {
        let mut h = seahash::SeaHasher::new();
        prompt.hash(&mut h);
        if self.non_deterministic {
            h.write_u64(rand::random());
        }
        // We're happy to chop the value here
        h.finish() as i32
    }
}

fn extract_md_kv_list(res: &str) -> Vec<(String, String)> {
    let re = Regex::new(r"\d+\.\s*([\w\s]+):\s*(.*)").unwrap();
    let mut items = Vec::new();

    for c in re.captures_iter(&res.replace("*", "")) {
        items.push((
            c.get(1).unwrap().as_str().trim().to_owned(),
            c.get(2).unwrap().as_str().trim().to_owned(),
        ));
    }

    items
}

fn extract_yaml<T: DeserializeOwned + Send>(res: &str) -> Result<T> {
    let re = Regex::new(r"(?s)```(?i:yaml)?(.*?)```").unwrap();

    if let Some(md_yaml) = re
        .captures(res)
        .map(|c| c.get(1).unwrap().as_str().to_string())
    {
        Ok(serde_yaml::from_str(&md_yaml)?)
    } else {
        Ok(serde_yaml::from_str(res)?)
    }
}

#[cfg(test)]
mod test {
    use bestiary::CreatureTemplate;
    use futures::StreamExt;

    use super::*;

    #[tokio::test]
    #[ignore = "ai"]
    async fn generate_sensible_creatures() {
        let client = AIClient::default();

        let minotaur = CreatureTemplate::stat_new(&client, "minotaur")
            .await
            .unwrap();
        assert!(minotaur.attributes.strength > minotaur.attributes.intelligence);
        assert!(minotaur.attributes.toughness > minotaur.attributes.willpower);

        let lich = CreatureTemplate::stat_new(&client, "lich").await.unwrap();
        assert!(lich.attributes.intelligence > lich.attributes.strength);
        assert!(lich.attributes.willpower > lich.attributes.toughness);
    }

    #[tokio::test]
    #[ignore = "ai"]
    async fn generate_places() {
        let client = AIClient::default();

        let places: Vec<(Place, HashMap<Location, Place>)> =
            place::generate_places(&client, &DUNGEON_PLACE_TYPE, 3)
                .await
                .collect()
                .await;

        println!("{places:?}");

        assert_eq!(places.len(), 3)
    }
}
