pub mod combat;
pub mod conversation;
pub mod core;
pub mod creatures;
pub mod dungeon;
pub mod player;
pub mod schema;

use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use dotenvy::dotenv;

use askama_actix::Template;

#[derive(Template)]
#[template(path = "index.html")]
struct Index {}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().expect(".env file not found");

    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(login)
            .service(actix_files::Files::new("/assets", "./assets"))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

#[get("/")]
async fn index() -> impl Responder {
    Index {}
}

#[get("/login")]
async fn login() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}
