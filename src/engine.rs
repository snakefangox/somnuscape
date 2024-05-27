use std::{rc::Rc, time::Duration};

use tokio::task::{self, LocalSet};

use crate::{
    commands::{self, Command}, connections::{EngineConnectionBroker, PlayerConnectionBroker}, PlayerEntry, Registry
};

/// All the engine state kept between frames, including world info
pub struct Engine {
    pub connection_broker: EngineConnectionBroker,
    pub player_registry: Registry<PlayerEntry>,
    pub commands: Vec<Rc<Command>>,
}

impl Engine {
    pub fn start_engine(player_registry: Registry<PlayerEntry>) -> PlayerConnectionBroker {
        let (player_connection_broker, connection_broker) = PlayerConnectionBroker::new();

        std::thread::spawn(move || {
            let local_set = LocalSet::new();
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let mud = Engine {
                player_registry,
                connection_broker,
                commands: commands::base_commands(),
            };

            task::Builder::new()
                .name("Engine")
                .spawn_local_on(run_engine(mud), &local_set)
                .unwrap();

            rt.block_on(local_set);
        });

        player_connection_broker
    }
}

async fn run_engine(mut mud: Engine) -> ! {
    let mut tick_timer = tokio::time::interval(Duration::from_secs_f64(1.0 / 20.0));
    loop {
        tick_timer.tick().await;

        mud.connection_broker.handle_connection_changes();

        while let Some((player, msg)) = mud.connection_broker.poll_player_messages() {
            let mut args_iter = msg.split_whitespace();
            if let Some(cmd) = args_iter.next() {
                let c = mud.commands.iter().find(|c| c.match_name(cmd)).cloned();
                let res = match c {
                    Some(cmd) => (cmd.cmd)(&mut mud, &mut args_iter),
                    None => String::new(),
                };

                mud.connection_broker.send_player_message(player, res);
            }
        }
    }
}
