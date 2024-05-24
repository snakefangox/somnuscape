mod engine;
mod state;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use engine::{Engine, PlayerConnection, PlayerConnectionHandler};
use futures::{SinkExt, StreamExt};
use nectar::{event::TelnetEvent, TelnetCodec};
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
};
use tokio_util::codec::Framed;

const STATE_DIR: &str = "somnustate/";
const PLAYER_REGISTRY_PATH: &str = "somnustate/player-registry.yaml";

#[tokio::main]
async fn main() -> Result<()> {
    console_subscriber::init();

    tokio::fs::create_dir_all(STATE_DIR).await?;
    let players: PlayerRegistry = PlayerRegistry::load_or_new().await?;
    let engine_msg_handler = Engine::start_engine(players.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], 5000));
    let listener = TcpListener::bind(addr).await?;

    loop {
        while let Ok((stream, _)) = listener.accept().await {
            let players = players.clone();
            let player_conn_handler = engine_msg_handler.clone();
            tokio::spawn(async move {
                let mut connection_state = ConnectionState::Unauthorized;
                let pch = player_conn_handler.clone();

                if let Err(e) = handler(stream, players, pch, &mut connection_state).await {
                    tracing::error!("Player session error: {}", e);
                }

                if let Some(player_id) = connection_state.get_player_id() {
                    player_conn_handler.end_connection(player_id);
                }
            });
        }
    }
}

#[tracing::instrument]
async fn handler(
    stream: TcpStream,
    player_registry: PlayerRegistry,
    engine_msg_handler: PlayerConnectionHandler,
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
                    .handle_login(player_msg, &player_registry, &engine_msg_handler)
                    .await;
                frame.send(TelnetEvent::Message(reply?)).await?;
            } else {
                break; // TODO: Better error handling?
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct PlayerRegistry(Arc<RwLock<Vec<PlayerEntry>>>);

impl PlayerRegistry {
    pub async fn load_or_new() -> anyhow::Result<Self> {
        let players = if tokio::fs::try_exists(PLAYER_REGISTRY_PATH)
            .await
            .is_ok_and(|r| r)
        {
            let yaml = tokio::fs::read_to_string(PLAYER_REGISTRY_PATH).await?;
            serde_yaml::from_str(&yaml)?
        } else {
            Vec::new()
        };

        Ok(PlayerRegistry(RwLock::new(players).into()))
    }

    pub async fn add_user(&self, player: PlayerEntry) -> Result<usize> {
        let mut write = self.0.write().await;
        let id = write.len();
        write.push(player);

        let yaml = serde_yaml::to_string::<Vec<PlayerEntry>>(write.as_ref())?;
        tokio::fs::write(PLAYER_REGISTRY_PATH, yaml).await?;

        Ok(id)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerEntry {
    username: String,
    password: u64,
}

#[derive(Debug, Default)]
pub enum ConnectionState {
    #[default]
    Unauthorized,
    NewUser(String),
    Login(usize),
    Authorized(usize, PlayerConnection),
}

impl ConnectionState {
    async fn handle_login(
        &mut self,
        msg: String,
        player_registry: &PlayerRegistry,
        player_connection_handler: &PlayerConnectionHandler,
    ) -> Result<String> {
        match self {
            ConnectionState::Unauthorized => {
                let username = msg.trim();
                let read = player_registry.0.read().await;
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
                    Ok(format!("Welcome {username}!\n\nWe haven't see you before, please choose a password!\n\
                    (Friendly reminder that for nostalgia's sake, your connection is unencrypted.\n\
                    *Please* use a unique password, people could be watching)\n\nPassword:"))
                }
            }
            ConnectionState::NewUser(username) => {
                let username = username.clone();
                let password = seahash::hash(msg.as_bytes());
                let player = PlayerEntry { username, password };
                let id = player_registry.add_user(player).await?;

                *self = Self::Authorized(id, player_connection_handler.setup_connection(id));

                Ok("Password set.\r\nWelcome to Somnuscape!".to_owned())
            }
            ConnectionState::Login(player_idx) => {
                let player_idx = *player_idx;
                let password = seahash::hash(msg.as_bytes());
                let read = player_registry.0.read().await;
                let player = &read[player_idx];

                if password == player.password {
                    *self = ConnectionState::Authorized(
                        player_idx,
                        player_connection_handler.setup_connection(player_idx),
                    );
                    Ok("Login successful.\nWelcome back to Somnuscape!".to_owned())
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
