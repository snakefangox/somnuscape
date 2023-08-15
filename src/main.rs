mod combat;
mod core;
mod creatures;
mod dungeon;
mod player;
mod routes;
mod web_types;

use std::{collections::HashSet, time::Duration};

use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, web, App, HttpServer};
use anyhow::Ok;
use base64::Engine;
use creatures::Creature;
use dotenvy::dotenv;
use dungeon::Dungeon;
use futures::executor::block_on;
use web_types::{PlayerAuthTransform, State};

use crate::core::Conversation;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv().expect(".env file not found");
    env_logger::init();

    let session_key = base64::prelude::BASE64_STANDARD.decode(std::env::var("SESSION_KEY")?)?;

    std::thread::spawn(Storyteller::run);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(State::new()))
            .wrap(PlayerAuthTransform)
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&session_key))
                    .cookie_secure(false)
                    .build(),
            )
            .service(routes::index)
            .service(routes::login)
            .service(routes::logout)
            .service(routes::character_creation)
            .service(routes::create_character)
            .service(routes::adventure)
            .service(routes::chat)
            .service(actix_files::Files::new("/assets", "./assets"))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
    .map_err(anyhow::Error::from)
}

/// How many times the janitor runs an hour
const STORYTELLER_FREQ: u64 = 60;

/// Creates and manages the world
pub struct Storyteller;

impl Storyteller {
    pub fn run() {
        let state = State::new();

        block_on(async {
            loop {
                let dungeons: HashSet<String> = state.list::<Dungeon>().await;
                if dungeons.len() < 10 {
                    let _ = Storyteller::generate_dungeons(&state).await;
                }

                std::thread::sleep(Duration::from_secs(3600 / STORYTELLER_FREQ))
            }
        });
    }

    async fn generate_dungeons(state: &State) -> anyhow::Result<()> {
        let mut dungeon_conv = Conversation::prime(include_str!("../primers/rooms.yaml"));
        let names = Storyteller::generate_names(10, "dungeon").await?;

        for name in names {
            let json = dungeon_conv
                .query(&format!("dungeon_name: {}", name))
                .await?
                .1;

            let (dungeon, creatures) = Dungeon::from_json(&json)?;
            for creature in creatures {
                let _ = Storyteller::get_creature(state, &creature).await;
            }
            state.set(&dungeon).await;
        }

        Ok(())
    }

    async fn generate_names(n: usize, name_type: &str) -> anyhow::Result<Vec<String>> {
        const SLACK: usize = 5;

        let mut name_conv = Conversation::prime(include_str!("../primers/names.yaml"));
        name_conv
            .say(&format!("{} fantasy {} names", n + SLACK, name_type))
            .await?;

        let best_names = name_conv
            .say("Order them from most interesting and unique to least interesting and unique")
            .await?
            .1;

        let r = regex::Regex::new(r"(?m)^[0-9]+\. (.*)$")?;
        let names: Vec<String> = r
            .captures_iter(&best_names)
            .filter_map(|c| c.get(1).map(|c| c.as_str().to_owned()))
            .take(n)
            .collect();
        Ok(names)
    }

    pub async fn get_creature(state: &State, name: &str) -> anyhow::Result<Creature> {
        if state.has::<Creature>(name).await {
            Ok(state.get(name).await.unwrap())
        } else {
            let result = serde_json::from_str::<Creature>(
                &Conversation::prime(include_str!("../primers/stats.yaml"))
                    .query(&format!("creature_name: {}", name))
                    .await?
                    .1,
            )?;
            state.set(&result).await;

            Ok(result)
        }
    }
}
