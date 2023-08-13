mod combat;
mod conversation;
mod core;
mod creatures;
mod dungeon;
mod player;

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Ok;
use base64::Engine;
use dotenvy::dotenv;

use askama_actix::Template;
use lazy_static::lazy_static;
use rand::Rng;
use redis::AsyncCommands;
use regex::Regex;
use serde::{Deserialize, Serialize};

lazy_static! {
    static ref USERNAME_RE: Regex = Regex::new(r"^[[:word:]]+$").unwrap();
}

#[derive(Template)]
#[template(path = "index.html")]
struct Index<'a> {
    error: &'a str,
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv().expect(".env file not found");
    env_logger::init();

    let state = State::new()?;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(index)
            .service(login)
            .service(actix_files::Files::new("/assets", "./assets"))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
    .map_err(anyhow::Error::from)
}

#[get("/")]
async fn index() -> impl Responder {
    Index { error: "" }
}

#[derive(Serialize, Deserialize)]
struct FormData {
    username: String,
    password: String,
}

#[post("/login")]
async fn login(state: web::Data<State>, form: web::Form<FormData>) -> impl Responder {
    if form.password.is_empty() {
        return HttpResponse::Ok().body(
            Index {
                error: "Password invalid, must be at least 1 character",
            }
            .render()
            .unwrap(),
        );
    } else if !USERNAME_RE.is_match(&form.username) {
        return HttpResponse::Ok().body(
            Index {
                error: "Character name invalid, must contain only letters, numbers and underscores",
            }
            .render()
            .unwrap(),
        );
    }

    if let Some(player) = state.get_player(&form.username).await {
        let hash = argon2rs::argon2i_simple(&form.password, &player.salt);
        if player.password == hash {
            return HttpResponse::Ok().body(format!(
                "Welcome back, {}, P: {}",
                &form.username, &form.password
            ));
        } else {
            return HttpResponse::Ok().body(
                Index {
                    error: "Incorrect password",
                }
                .render()
                .unwrap(),
            );
        }
    } else {
        let salt: [u8; 16] = rand::thread_rng().gen();
        let salt = base64::prelude::BASE64_STANDARD.encode(salt);
        let hash = argon2rs::argon2i_simple(&form.password, &salt);

        state
            .set_player(player::Player::new(form.username.clone(), hash, salt))
            .await;
        return HttpResponse::Ok().body(format!("Heyo, {}, P: {}", &form.username, &form.password));
    }
}

#[derive(Debug, Clone)]
pub struct State {
    client: redis::Client,
}

impl State {
    pub fn new() -> anyhow::Result<Self> {
        let client = redis::Client::open(std::env::var("REDIS_URL")?)?;
        Ok(Self { client })
    }

    pub async fn con(&self) -> redis::aio::Connection {
        self.client
            .get_async_connection()
            .await
            .expect("Redis should be available")
    }

    pub async fn get_player(&self, name: &str) -> Option<player::Player> {
        let json: String = self.con().await.get(format!("player:{name}")).await.ok()?;
        serde_json::from_str(&json).ok()
    }

    pub async fn set_player(&self, player: player::Player) {
        let name = &player.name;
        self.con()
            .await
            .set::<String, String, ()>(
                format!("player:{name}"),
                serde_json::to_string(&player).unwrap(),
            )
            .await
            .unwrap();
    }
}
