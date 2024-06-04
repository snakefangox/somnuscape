mod commands;
mod config;
mod connections;
mod engine;
mod generation;
mod mud;
mod state;

use std::net::SocketAddr;
use std::{error::Error, fmt::Display};

use anyhow::Result;
use connections::{EngineConnection, PlayerConnectionBroker};
use engine::Engine;
use futures::{SinkExt, StreamExt};
use generation::Generator;
use mud::world::Location;
use nectar::{event::TelnetEvent, TelnetCodec};
use serde::{Deserialize, Serialize};
use state::Registry;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;

#[tokio::main]
async fn main() -> Result<()> {
    console_subscriber::init();

    let players: Registry<PlayerEntry> = Registry::load_or_new("player-registry.yaml").await?;
    let (mut gen, gen_handle) = Generator::new();
    tokio::spawn(async move {
        gen.run().await;
    });

    let engine_msg_handler = Engine::start_engine(players.clone(), gen_handle);

    let addr: SocketAddr = config::get()
        .server_address
        .parse()
        .expect("Server address should be a valid <ip:port>");
    let listener = TcpListener::bind(addr).await?;

    loop {
        while let Ok((stream, addr)) = listener.accept().await {
            tracing::info!("Player connected from {addr}");

            let players = players.clone();
            let player_conn_handler = engine_msg_handler.clone();

            tokio::spawn(async move {
                let mut connection_state = ConnectionState::Unauthorized;
                let pch = player_conn_handler.clone();

                if let Err(e) = handler(stream, players.clone(), pch, &mut connection_state).await {
                    if e.is::<AppErrors>() {
                        if let Ok(AppErrors::PlayerDisconnected(id)) = e.downcast::<AppErrors>() {
                            let pr = players.read().await;
                            let n = pr.get(id).map(|p| p.username.as_str()).unwrap_or_default();
                            tracing::info!("Player disconnected: {n}");
                        }
                    } else {
                        tracing::error!("Player connection error: {e}");
                    }
                }

                if let Some(player_id) = connection_state.get_player_id() {
                    player_conn_handler.end_connection(player_id);
                }
            });
        }
    }
}

async fn handler(
    stream: TcpStream,
    player_registry: Registry<PlayerEntry>,
    broker: PlayerConnectionBroker,
    connection_state: &mut ConnectionState,
) -> Result<()> {
    let mut frame = Framed::new(stream, TelnetCodec::new(1024));

    frame
        .send(TelnetEvent::Message(
            "<~~ Welcome adventurer! What is thy name? ~~>".to_string(),
        ))
        .await?;

    loop {
        if let ConnectionState::Authorized(_, ref mut handler) = connection_state {
            tokio::select! {
                Some(Ok(TelnetEvent::Message(player_msg))) = frame.next() => {
                    handler.send(player_msg)?;
                }
                response = handler.recv() => {
                    frame.send(TelnetEvent::Message(response?)).await?;
                }
            }
        } else {
            if let Some(Ok(TelnetEvent::Message(player_msg))) = frame.next().await {
                let reply = connection_state
                    .handle_login(player_msg, &player_registry, &broker)
                    .await;
                frame.send(TelnetEvent::Message(reply?)).await?;
            } else {
                break; // TODO: Better error handling?
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerEntry {
    pub username: String,
    pub password: u64,
}

#[derive(Debug, Default)]
pub enum ConnectionState {
    #[default]
    Unauthorized,
    NewUser(String),
    Login(usize),
    Authorized(usize, EngineConnection),
}

impl ConnectionState {
    async fn handle_login(
        &mut self,
        msg: String,
        player_registry: &Registry<PlayerEntry>,
        broker: &PlayerConnectionBroker,
    ) -> Result<String> {
        match self {
            ConnectionState::Unauthorized => {
                let username = msg.trim();
                let read = player_registry.read().await;
                let player = read
                    .iter()
                    .enumerate()
                    .find(|(_, p)| p.username.to_lowercase() == username.to_lowercase());

                if let Some((idx, player)) = player {
                    *self = ConnectionState::Login(idx);
                    Ok(format!(
                        "Welcome back {}!\r\nPlease enter your password:",
                        player.username
                    ))
                } else {
                    *self = ConnectionState::NewUser(username.to_string());
                    Ok(format!("Welcome {username}!\r\n\r\nWe haven't see you before, please choose a password!\r\n\
                    (Friendly reminder that for nostalgia's sake, your connection is unencrypted.\r\n\
                    *Please* use a unique password, people could be watching)\r\n\r\nPassword:"))
                }
            }
            ConnectionState::NewUser(username) => {
                let username = username.clone();
                let password = seahash::hash(msg.as_bytes());
                let player = PlayerEntry { username, password };

                tracing::info!("Player {} registered an account", player.username);
                let id = player_registry.add_user(player).await?;

                *self = Self::Authorized(id, broker.setup_connection(id));

                Ok("Password set.\r\nWelcome to Somnuscape!".to_owned())
            }
            ConnectionState::Login(player_id) => {
                let player_id = *player_id;
                let password = seahash::hash(msg.as_bytes());
                let read = player_registry.read().await;
                let player = &read[player_id];

                if password == player.password {
                    *self =
                        ConnectionState::Authorized(player_id, broker.setup_connection(player_id));
                    tracing::info!("Player {} logged in", player.username);

                    Ok("Login successful.\r\nWelcome back to Somnuscape!".to_owned())
                } else {
                    Ok(format!(
                        "Login failed, retry your password for {}:",
                        player.username
                    ))
                }
            }
            ConnectionState::Authorized(_, _) => {
                unreachable!("Should not be handling login if already logged in");
            }
        }
    }

    pub fn get_player_id(&self) -> Option<usize> {
        match self {
            ConnectionState::Unauthorized => None,
            ConnectionState::NewUser(_) => None,
            ConnectionState::Login(_) => None,
            ConnectionState::Authorized(id, _) => Some(*id),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum AppErrors {
    PlayerDisconnected(usize),
    TooManyConnections(Location),
    AIStructureError,
}

impl Error for AppErrors {}

impl Display for AppErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppErrors::PlayerDisconnected(_) => f.write_str("Player Disconnected"),
            AppErrors::TooManyConnections(l) => {
                f.write_fmt(format_args!("To many connections to place {l:?}"))
            }
            AppErrors::AIStructureError => f.write_str("AI Structure Error"),
        }
    }
}

mod filters {
    pub fn pluralize<T: std::fmt::Display>(s: T) -> ::askama::Result<String> {
        let mut s = s.to_string();
        if s.ends_with("s") {
            s.push_str("es");
            Ok(s)
        } else {
            s.push_str("s");
            Ok(s)
        }
    }
}
