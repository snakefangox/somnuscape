use std::{
    collections::{HashMap, HashSet},
    future::{ready, Ready},
    rc::Rc,
    sync::Mutex,
    time::{Duration, Instant},
};

use actix::{Actor, ActorContext, AsyncContext, Handler, Message, StreamHandler, WeakAddr};
use actix_session::SessionExt;
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, FromRequest, HttpMessage, HttpResponse,
};
use actix_web_actors::ws;
use askama_actix::Template;
use futures::{future::LocalBoxFuture, FutureExt};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::player::Player;

pub const USERNAME: &str = "username";

pub trait Keyed {
    /// The namespace to use for the resource, should be plural (players:{name} not player:{name})
    fn get_key() -> &'static str;
    /// Gets a unique (among resources of that type) name for a specific instance of the resource
    fn name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct State {
    client: redis::Client,
}

fn get_redis_client() -> redis::Client {
    redis::Client::open(std::env::var("REDIS_URL").expect("REDIS_URL should be set"))
        .expect("Redis URL should be valid")
}

impl State {
    pub fn new() -> Self {
        let client = get_redis_client();
        Self { client }
    }

    pub async fn con(&self) -> redis::aio::Connection {
        self.client
            .get_async_connection()
            .await
            .expect("Redis should be available")
    }

    pub async fn get<T>(&self, name: &str) -> Option<T>
    where
        T: for<'a> Deserialize<'a> + Keyed,
    {
        let key = T::get_key();
        let json: String = self.con().await.get(format!("{key}:{name}")).await.ok()?;
        serde_json::from_str::<T>(&json).ok()
    }

    pub async fn grab<T>(&self, name: &str) -> T
    where
        T: for<'a> Deserialize<'a> + Keyed,
    {
        self.get::<T>(name).await.unwrap()
    }

    pub async fn set<T>(&self, val: &T)
    where
        T: Serialize + Keyed,
    {
        let mut con = self.con().await;
        let key = T::get_key();
        let name = val.name();
        con.set::<String, String, ()>(
            format!("{key}:{name}"),
            serde_json::to_string(&val).unwrap(),
        )
        .await
        .unwrap();
        con.sadd::<&str, &str, ()>(key, name).await.unwrap();
    }

    pub async fn list<T>(&self) -> HashSet<String>
    where
        T: Keyed,
    {
        self.con()
            .await
            .smembers(T::get_key())
            .await
            .unwrap_or_default()
    }

    pub async fn has<T>(&self, name: &str) -> bool
    where
        T: Keyed,
    {
        self.con()
            .await
            .sismember::<&str, &str, bool>(T::get_key(), name)
            .await
            .unwrap()
    }

    pub async fn add_user(&self, name: String, password: [u8; 32], salt: String) {
        self.set(&Player::new(name.clone())).await;
        self.set(&UserCreds {
            name,
            password,
            salt,
        })
        .await;
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserCreds {
    pub name: String,
    pub salt: String,
    pub password: [u8; 32],
}

impl Keyed for UserCreds {
    fn get_key() -> &'static str {
        "usercreds"
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl FromRequest for Player {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let value = req.extensions().get::<Player>().cloned();
        let result = match value {
            Some(v) => Ok(v),
            None => Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied).into()),
        };
        ready(result)
    }
}

pub struct PlayerAuthTransform;

impl<S, B> Transform<S, ServiceRequest> for PlayerAuthTransform
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = PlayerAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(PlayerAuthMiddleware {
            service: Rc::new(service),
            state: Rc::new(State::new()),
        }))
    }
}

pub struct PlayerAuthMiddleware<S> {
    service: Rc<S>,
    state: Rc<State>,
}

impl<S, B> Service<ServiceRequest> for PlayerAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let state = self.state.clone();
        let service = self.service.clone();

        async move {
            let session = req.get_session();
            let auth = if let Some(username) = session.get::<String>(USERNAME)? {
                if let Some(player) = state.get::<Player>(&username).await {
                    req.extensions_mut().insert(player);
                    true
                } else {
                    session.remove(USERNAME);
                    false
                }
            } else {
                false
            };

            if !auth && ["/adventure", "/character_creation"].contains(&req.path()) {
                return make_redirect(req, "/");
            }

            if auth && req.path() == "/" {
                return make_redirect(req, "/adventure");
            }

            Ok(service
                .call(req)
                .await
                .map(ServiceResponse::map_into_left_body)?)
        }
        .boxed_local()
    }
}

fn make_redirect<B>(
    req: ServiceRequest,
    redirect: &str,
) -> Result<ServiceResponse<EitherBody<B>>, Error> {
    Ok(req.into_response(
        HttpResponse::TemporaryRedirect()
            .append_header(("Location", redirect))
            .finish()
            .map_into_right_body(),
    ))
}

pub struct LogWebsocket {
    heartbeat: Instant,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LogMessage(pub String);

#[derive(Template)]
#[template(path = "elements/log.html")]
struct Log<'a> {
    entry: &'a str,
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

impl LogWebsocket {
    pub fn new() -> Self {
        LogWebsocket {
            heartbeat: Instant::now(),
        }
    }

    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.heartbeat) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Actor for LogWebsocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
    }
}

impl Handler<LogMessage> for LogWebsocket {
    type Result = ();

    fn handle(&mut self, msg: LogMessage, ctx: &mut Self::Context) {
        ctx.text(Log {entry: &msg.0 }.render().unwrap());
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for LogWebsocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
                self.heartbeat = Instant::now();
            }
            Ok(ws::Message::Pong(_)) => self.heartbeat = Instant::now(),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => (),
        }
    }
}

#[derive(Debug, Default)]
pub struct WebsocketMap {
    pub data: Mutex<HashMap<String, WeakAddr<LogWebsocket>>>,
}
