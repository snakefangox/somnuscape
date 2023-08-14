use actix_session::Session;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder, Result};
use askama_actix::Template;
use base64::Engine;
use lazy_static::lazy_static;
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    player::Player,
    web_types::{State, USERNAME},
};

lazy_static! {
    static ref USERNAME_RE: Regex = Regex::new(r"^[[:word:]]{1,64}$").unwrap();
}

#[derive(Template)]
#[template(path = "index.html")]
struct Index<'a> {
    error: &'a str,
}

#[derive(Template)]
#[template(path = "character_creation.html")]
struct CharacterCreation<'a> {
    name: &'a str,
}

#[derive(Template)]
#[template(path = "adventure.html")]
struct Adventure<'a> {
    name: &'a str,
}

#[get("/")]
async fn index(req: HttpRequest) -> Result<impl Responder> {
    Ok(Index { error: "" }.respond_to(&req))
}

#[derive(Serialize, Deserialize)]
struct LoginFormData {
    username: String,
    password: String,
}

#[post("/login")]
async fn login(
    state: web::Data<State>,
    form: web::Form<LoginFormData>,
    session: Session,
) -> impl Responder {
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
                error: "Character name invalid, must contain only letters, numbers and underscores and be between 1 and 64 characters",
            }
            .render()
            .unwrap(),
        );
    }

    if let Some(player) = state.get_player(&form.username).await {
        let hash = argon2rs::argon2i_simple(&form.password, &player.salt);
        if player.password == hash {
            session.insert(USERNAME, &form.username).unwrap();

            return HttpResponse::SeeOther()
                .append_header(("Location", "/adventure"))
                .body(());
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
            .set_player(Player::new(form.username.clone(), hash, salt))
            .await;
        session.insert(USERNAME, &form.username).unwrap();
        return HttpResponse::SeeOther()
            .append_header(("Location", "/character_creation"))
            .body(());
    }
}

#[get("/logout")]
async fn logout(session: Session) -> Result<impl Responder> {
    session.remove(USERNAME);
    Ok(HttpResponse::TemporaryRedirect()
        .append_header(("Location", "/"))
        .body(()))
}

#[get("/character_creation")]
async fn character_creation(
    state: web::Data<State>,
    player: Player,
    req: HttpRequest,
) -> Result<impl Responder> {
    if player.creation_complete() {
        return Ok(HttpResponse::SeeOther()
            .append_header(("Location", "/adventure"))
            .body(()));
    }

    return Ok(CharacterCreation { name: &player.name }.respond_to(&req));
}

#[derive(Serialize, Deserialize)]
struct CharacterCreateFormData {
    strength: u32,
    agility: u32,
    intelligence: u32,
}

#[post("/create_character")]
async fn create_character(
    state: web::Data<State>,
    player: Player,
    req: HttpRequest,
    form: web::Form<CharacterCreateFormData>,
) -> Result<impl Responder> {
    if player.creation_complete() {
        return Ok(HttpResponse::SeeOther()
            .append_header(("Location", "/adventure"))
            .body(()));
    }

    println!("{}, {}, {}", form.strength, form.agility, form.intelligence);

    return Ok(CharacterCreation { name: &player.name }.respond_to(&req));
}

#[get("/adventure")]
async fn adventure(
    state: web::Data<State>,
    player: Player,
    req: HttpRequest,
) -> Result<impl Responder> {
    if !player.creation_complete() {
        return Ok(HttpResponse::SeeOther()
            .append_header(("Location", "/character_creation"))
            .body(()));
    }

    return Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/adventure"))
        .body(()));
}
