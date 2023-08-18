use std::collections::VecDeque;

use actix_session::Session;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder, Result};
use askama_actix::Template;
use base64::Engine;
use lazy_static::lazy_static;
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    core::{self, AttributeRating, ChatContext},
    player::Player,
    web_types::{State, UserCreds, USERNAME},
};

lazy_static! {
    static ref USERNAME_RE: Regex = Regex::new(r"^[[:word:]]{1,64}$").unwrap();
}

#[derive(Template)]
#[template(path = "index.html")]
struct Index<'a> {
    error: &'a str,
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

    if let Some(user) = state.get::<UserCreds>(&form.username).await {
        let hash = argon2rs::argon2i_simple(&form.password, &user.salt);
        if user.password == hash {
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

        session.insert(USERNAME, &form.username).unwrap();
        state.add_user(form.username.clone(), hash, salt).await;

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

#[derive(Template)]
#[template(path = "character_creation.html")]
struct CharacterCreation<'a> {
    name: &'a str,
}

#[get("/character_creation")]
async fn character_creation(player: Player, req: HttpRequest) -> Result<impl Responder> {
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
    mut player: Player,
    form: web::Form<CharacterCreateFormData>,
) -> Result<impl Responder> {
    if player.creation_complete() {
        return Ok(HttpResponse::SeeOther()
            .append_header(("Location", "/adventure"))
            .body(()));
    }

    let total = form.strength + form.agility + form.intelligence;
    let s = AttributeRating::from_rank(form.strength);
    let a = AttributeRating::from_rank(form.agility);
    let i = AttributeRating::from_rank(form.intelligence);
    if total > core::STARTING_POINT_TOTAL || s.is_none() || a.is_none() || i.is_none() {
        return Ok(HttpResponse::SeeOther()
            .append_header(("Location", "/create_character"))
            .body(()));
    }

    player.finish_character_creation(s.unwrap(), a.unwrap(), i.unwrap());
    state.set(&player).await;

    return Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/adventure"))
        .body(()));
}

#[derive(Template)]
#[template(path = "adventure.html")]
struct Adventure<'a> {
    name: &'a str,
    p: &'a Player,
    messages: &'a VecDeque<(String, String)>,
    actions: &'a str,
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

    let chat_ctx: ChatContext = state
        .get(&player.location.name())
        .await
        .unwrap_or_else(|| ChatContext::new(&player.location));

    return Ok(Adventure {
        name: &player.name,
        p: &player,
        messages: chat_ctx.messages(),
        actions: &crate::action::get_active_actions(&player),
    }
    .respond_to(&req));
}

#[derive(Serialize, Deserialize)]
struct ChatFormData {
    message: String,
}

#[derive(Template)]
#[template(path = "elements/chatbox.html")]
struct ChatMsgs<'a> {
    messages: &'a VecDeque<(String, String)>,
}

#[post("/chat")]
async fn chat(
    mut form: web::Form<ChatFormData>,
    state: web::Data<State>,
    player: Player,
    req: HttpRequest,
) -> Result<impl Responder> {
    let location = &player.location;
    let mut chat_ctx: ChatContext = state
        .get(&location.name())
        .await
        .unwrap_or_else(|| ChatContext::new(location));

    chat_ctx.send_msg(&player, std::mem::take(&mut form.message));
    state.set(&chat_ctx).await;

    Ok(ChatMsgs {
        messages: chat_ctx.messages(),
    }
    .respond_to(&req))
}

#[post("/action/{action_name}")]
async fn action(
    action: web::Path<String>,
    state: web::Data<State>,
    player: Player,
    req: HttpRequest,
) -> Result<impl Responder> {
    Ok(crate::action::get_input_menu(&player, &state, &action)
        .await
        .respond_to(&req))
}
