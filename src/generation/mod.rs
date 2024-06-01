mod place;
mod bestiary;

use ollama_rs::{
    generation::{completion::request::GenerationRequest, options::GenerationOptions},
    Ollama,
};
use regex::Regex;
use serde::de::DeserializeOwned;
use anyhow::Result;

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
