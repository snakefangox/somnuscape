mod bestiary;
mod place;

use anyhow::Result;
use crossbeam::channel::{Receiver, Sender, TryRecvError};
use ollama_rs::{
    generation::{completion::request::GenerationRequest, options::GenerationOptions},
    Ollama,
};
use regex::Regex;
use serde::de::DeserializeOwned;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::world::{Place, PlaceType};

#[derive(Debug)]
pub struct Generator {
    request_queue: UnboundedReceiver<GenerationReq>,
    response_queue: Sender<GenerationRes>,
    client: Ollama,
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
    Place(Vec<Place>),
}

impl Generator {
    pub fn new() -> (Self, GeneratorHandle) {
        let (req_s, req_r) = tokio::sync::mpsc::unbounded_channel();
        let (res_s, res_r) = crossbeam::channel::unbounded();

        (
            Self {
                request_queue: req_r,
                response_queue: res_s,
                client: Ollama::default(),
            },
            GeneratorHandle {
                request_queue: req_s,
                response_queue: res_r,
            },
        )
    }

    pub async fn run(&mut self) {
        loop {
            let req = self
                .request_queue
                .recv()
                .await
                .expect("Gen request channel shouldn't close");
            let res = match req {
                GenerationReq::Places(place_type, count) => {
                    GenerationRes::Place(place::generate_places(place_type, count).await)
                }
            };

            self.response_queue
                .send(res)
                .expect("Gen response channel shouldn't close");
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
            Err(TryRecvError::Disconnected) => unreachable!("Gen handle response channel shouldn't close"),
        }
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

async fn generate(prompt: String) -> Result<String> {
    let ollama = Ollama::default();
    Ok(ollama
        .generate(
            GenerationRequest::new("llama3:latest".to_string(), prompt)
                .options(GenerationOptions::default().seed(rand::random())),
        )
        .await?
        .response)
}
