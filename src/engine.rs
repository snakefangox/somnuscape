use std::collections::HashMap;

use anyhow::anyhow;
use futures::StreamExt;
use mlua::prelude::*;
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::LocalSet,
};
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamMap};

use crate::PlayerRegistry;

pub type MudMessage = String;

#[derive(Debug)]
pub struct Engine {
    _fennel: Lua,
    _player_registry: PlayerRegistry,
}

impl Engine {
    pub fn start_engine(_player_registry: PlayerRegistry) -> PlayerConnectionHandler {
        let (handler, mut receiver) = PlayerConnectionHandler::new();

        std::thread::spawn(move || {
            let local = LocalSet::new();
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let mut player_connections: HashMap<usize, UnboundedSender<MudMessage>> =
                HashMap::new();
            let mut incoming_msgs: StreamMap<usize, UnboundedReceiverStream<MudMessage>> =
                StreamMap::new();

            let mut mud = Engine {
                _fennel: Lua::new(),
                _player_registry,
            };

            local.spawn_local(async move {
                loop {
                    tokio::select! {
                        conn_msg = handle_new_connections(&mut receiver) => {
                            match conn_msg.unwrap() {
                                PlayerConnectMsg::Connect(PlayerConnection(id, r, s)) => {
                                    player_connections.insert(id, s);
                                    incoming_msgs.insert(id, UnboundedReceiverStream::new(r));
                                },
                                PlayerConnectMsg::Disconnect(id) => {
                                    player_connections.remove(&id);
                                    incoming_msgs.remove(&id);
                                },
                            }
                        }
                        Some((id, msg)) = incoming_msgs.next() => {
                            // TODO: More advanced handling
                            for chan in player_connections.values() {
                                let _ = chan.send(msg.clone());
                            }
                        }
                    }
                }
            });

            rt.block_on(local);
        });

        handler
    }
}

async fn handle_new_connections(
    receiver: &mut UnboundedReceiver<PlayerConnectMsg>,
) -> anyhow::Result<PlayerConnectMsg> {
    receiver
        .recv()
        .await
        .ok_or(anyhow!("Engine channel closed"))
}

async fn handle_messages() -> Option<()> {
    None
}

#[derive(Debug)]
pub struct PlayerConnection(
    usize,
    UnboundedReceiver<MudMessage>,
    UnboundedSender<MudMessage>,
);

impl PlayerConnection {
    pub fn send(&mut self, msg: MudMessage) -> anyhow::Result<()> {
        self.2.send(msg)?;
        Ok(())
    }

    pub async fn recv(&mut self) -> anyhow::Result<MudMessage> {
        self.1
            .recv()
            .await
            .ok_or(anyhow!("Could not get message before channel closed"))
    }
}

#[derive(Debug, Clone)]
pub struct PlayerConnectionHandler(UnboundedSender<PlayerConnectMsg>);

pub enum PlayerConnectMsg {
    Connect(PlayerConnection),
    Disconnect(usize),
}

impl PlayerConnectionHandler {
    pub fn new() -> (PlayerConnectionHandler, UnboundedReceiver<PlayerConnectMsg>) {
        let (s, r) = tokio::sync::mpsc::unbounded_channel::<PlayerConnectMsg>();
        (PlayerConnectionHandler(s), r)
    }

    pub fn setup_connection(&self, player_id: usize) -> PlayerConnection {
        let (s_engine, r_engine) = tokio::sync::mpsc::unbounded_channel();
        let (s_player, r_player) = tokio::sync::mpsc::unbounded_channel();
        self.0
            .send(PlayerConnectMsg::Connect(PlayerConnection(
                player_id, r_engine, s_player,
            )))
            .expect("Message send shouldn't error");
        PlayerConnection(player_id, r_player, s_engine)
    }

    pub fn end_connection(&self, player_id: usize) {
        self.0.send(PlayerConnectMsg::Disconnect(player_id));
    }
}
