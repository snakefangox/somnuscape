use std::time::Duration;

use tokio::task::{self, LocalSet};

use crate::{
    connections::{EngineConnectionBroker, PlayerConnectionBroker},
    PlayerEntry, Registry,
};

#[derive(Debug)]
pub struct Engine {
    connection_broker: EngineConnectionBroker,
    player_registry: Registry<PlayerEntry>,
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
            mud.connection_broker.send_player_message(player, msg);
        }
    }
}
