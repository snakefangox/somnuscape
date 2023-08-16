mod combat;
mod core;
mod dungeon;
mod player;
mod routes;
mod web_types;
mod worldbuilding;


use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, web, App, HttpServer};
use base64::Engine;
use dotenvy::dotenv;
use web_types::{PlayerAuthTransform, State};


#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv().expect(".env file not found");
    env_logger::init();

    let session_key = base64::prelude::BASE64_STANDARD.decode(std::env::var("SESSION_KEY")?)?;

    tokio::spawn(worldbuilding::run());

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
