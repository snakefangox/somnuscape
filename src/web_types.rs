use std::{
    future::{ready, Ready},
    rc::Rc,
};

use actix_session::SessionExt;
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, FromRequest, HttpMessage, HttpResponse,
};
use futures::{future::LocalBoxFuture, FutureExt};
use redis::AsyncCommands;

use crate::player::Player;

pub const USERNAME: &str = "username";

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

    pub async fn get_player(&self, name: &str) -> Option<Player> {
        let json: String = self.con().await.get(format!("player:{name}")).await.ok()?;
        serde_json::from_str(&json).ok()
    }

    pub async fn set_player(&self, player: Player) {
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
                if let Some(player) = state.get_player(&username).await {
                    req.extensions_mut().insert(player);
                    true
                } else {
                    session.remove(USERNAME);
                    false
                }
            } else {
                false
            };

            if !auth && (!["/", "/login"].contains(&req.path())) {
                return Ok(req.into_response(
                    HttpResponse::TemporaryRedirect()
                        .append_header(("Location", "/"))
                        .finish()
                        .map_into_right_body(),
                ));
            }

            let res = service
                .call(req)
                .await
                .map(ServiceResponse::map_into_left_body)?;
            Ok(res)
        }
        .boxed_local()
    }
}
