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
const FENNEL_MOD: &str = "fennel";

#[derive(Debug)]
pub struct Engine {
    fennel: Lua,
    player_registry: PlayerRegistry,
}

impl Engine {
    pub fn start_engine(player_registry: PlayerRegistry) -> PlayerConnectionHandler {
        let (handler, receiver) = PlayerConnectionHandler::new();

        std::thread::spawn(move || {
            let local = LocalSet::new();
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let player_connections: HashMap<usize, UnboundedSender<MudMessage>> = HashMap::new();
            let incoming_msgs: StreamMap<usize, UnboundedReceiverStream<MudMessage>> =
                StreamMap::new();

            let fennel = create_fennel().expect("Failed to load fennel");

            let mud = Engine {
                fennel,
                player_registry,
            };

            local.spawn_local(run_engine(receiver, player_connections, incoming_msgs, mud));

            rt.block_on(local);
        });

        handler
    }

    fn run_command(&self, cmd: &str) -> LuaResult<String> {
        let mut args = cmd.split_whitespace();
        if let Some(cmd) = args.next() {
            let args: Vec<&str> = args.collect();
            let run_cmd: LuaFunction = self.fennel.globals().get("run-cmd")?;
            let response: LuaString = run_cmd.call((cmd, "world", "player", args))?;
            Ok(response.to_string_lossy().to_string())
        } else {
            Ok(format!("Command not recognized: '{cmd}'"))
        }
    }
}

async fn run_engine(
    mut receiver: UnboundedReceiver<PlayerConnectMsg>,
    mut player_connections: HashMap<usize, UnboundedSender<String>>,
    mut incoming_msgs: StreamMap<usize, UnboundedReceiverStream<String>>,
    mud: Engine,
) -> ! {
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
            Some((sender_id, msg)) = incoming_msgs.next() => {
                // TODO: More advanced handling
                let res = mud.run_command(&msg).unwrap();
                let _ = player_connections[&sender_id].send(res);
            }
        }
    }
}

fn create_fennel() -> anyhow::Result<Lua> {
    let fennel = Lua::new();

    {
        let fnl_fn = fennel
            .load(include_str!("include/fennel.lua"))
            .into_function()?;
        let fnl: LuaTable = fennel.load_from_function(FENNEL_MOD, fnl_fn)?;
        fnl.call_function("install", ())?;
        fennel.globals().raw_set(FENNEL_MOD, fnl.clone())?;

        fnl.call_function(
            "eval",
            fennel.create_string(include_str!("include/core.fnl"))?,
        )?;
    }

    Ok(fennel)
}

async fn handle_new_connections(
    receiver: &mut UnboundedReceiver<PlayerConnectMsg>,
) -> anyhow::Result<PlayerConnectMsg> {
    receiver
        .recv()
        .await
        .ok_or(anyhow!("Engine channel closed"))
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
        self.0
            .send(PlayerConnectMsg::Disconnect(player_id))
            .expect("Engine message send shouldn't error");
    }
}
