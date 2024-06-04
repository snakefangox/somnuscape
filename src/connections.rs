use std::collections::HashMap;

use crossbeam::channel::Receiver;
use crossbeam::channel::Sender;
use tokio::sync::mpsc::UnboundedReceiver as TokioReceiver;
use tokio::sync::mpsc::UnboundedSender as TokioSender;

use crate::state::PlayerId;
use crate::AppErrors;

pub type MudMessage = String;

/// The connection object the engine holds to talk to the player
#[derive(Debug, Clone)]
pub struct PlayerConnection(
    pub PlayerId,
    pub Receiver<MudMessage>,
    pub TokioSender<MudMessage>,
);

impl PlayerConnection {
    pub fn poll(&mut self) -> anyhow::Result<MudMessage> {
        Ok(self.1.try_recv()?)
    }
}

/// The connection object the player task holds to talk to the engine
#[derive(Debug)]
pub struct EngineConnection(PlayerId, TokioReceiver<MudMessage>, Sender<MudMessage>);

impl EngineConnection {
    pub fn send(&mut self, msg: MudMessage) -> anyhow::Result<()> {
        self.2.send(msg)?;
        Ok(())
    }

    pub async fn recv(&mut self) -> anyhow::Result<MudMessage> {
        let msg = self.1.recv().await;
        match msg {
            Some(m) => Ok(m),
            None => Err(AppErrors::PlayerDisconnected(self.0).into()),
        }
    }
}

pub enum PlayerConnectMsg {
    Connect(PlayerConnection),
    Disconnect(PlayerId),
}

#[derive(Debug, Clone)]
pub struct PlayerConnectionBroker(Sender<PlayerConnectMsg>);

impl PlayerConnectionBroker {
    pub fn new() -> (PlayerConnectionBroker, EngineConnectionBroker) {
        let (s, r) = crossbeam::channel::unbounded::<PlayerConnectMsg>();
        (PlayerConnectionBroker(s), EngineConnectionBroker::new(r))
    }

    pub fn setup_connection(&self, player_id: PlayerId) -> EngineConnection {
        let (s_engine, r_engine) = crossbeam::channel::unbounded();
        let (s_player, r_player) = tokio::sync::mpsc::unbounded_channel();
        self.0
            .send(PlayerConnectMsg::Connect(PlayerConnection(
                player_id, r_engine, s_player,
            )))
            .expect("Join message send to engine shouldn't error");
        EngineConnection(player_id, r_player, s_engine)
    }

    pub fn end_connection(&self, player_id: PlayerId) {
        self.0
            .send(PlayerConnectMsg::Disconnect(player_id))
            .expect("Disconnect message send to engine shouldn't error");
    }
}

#[derive(Debug, Clone)]
pub struct EngineConnectionBroker {
    incoming_connections: Receiver<PlayerConnectMsg>,
    player_connections: HashMap<PlayerId, PlayerConnection>,
}

impl EngineConnectionBroker {
    fn new(incoming_connections: Receiver<PlayerConnectMsg>) -> Self {
        Self {
            incoming_connections,
            player_connections: HashMap::new(),
        }
    }

    pub fn handle_connection_changes(&mut self) {
        while let Ok(msg) = self.incoming_connections.try_recv() {
            match msg {
                PlayerConnectMsg::Connect(player_connection) => self
                    .player_connections
                    .insert(player_connection.0, player_connection),
                PlayerConnectMsg::Disconnect(player_id) => {
                    self.player_connections.remove(&player_id)
                }
            };
        }
    }

    pub fn poll_player_messages(&mut self) -> Option<(PlayerId, MudMessage)> {
        for player_connection in self.player_connections.values_mut() {
            if let Ok(msg) = player_connection.poll() {
                return Some((player_connection.0, msg));
            }
        }

        None
    }

    pub fn send_player_message(&mut self, player: PlayerId, msg: MudMessage) {
        if let Some(player_connection) = self.player_connections.get(&player) {
            if let Err(e) = player_connection.2.send(msg) {
                tracing::error!("Error sending to player {e}");
            }
        }
    }

    pub fn disconnect_player(&mut self, player: PlayerId) {
        self.player_connections.remove(&player);
    }
}
