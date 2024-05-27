use std::{rc::Rc, time::Duration};

use crate::{
    commands::{self, Command},
    connections::{EngineConnectionBroker, PlayerConnectionBroker},
    PlayerEntry, Registry,
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
            let mud = Engine {
                player_registry,
                connection_broker,
                commands: commands::base_commands(),
            };

            run_engine(mud);
        });

        player_connection_broker
    }
}

fn run_engine(mut mud: Engine) -> ! {
    let tick_period = Duration::from_secs_f64(1.0 / 20.0);
    let tick_duration = crossbeam::channel::tick(tick_period);

    loop {
        tick_duration.recv().expect("Tick channel should not close");

        mud.connection_broker.handle_connection_changes();

        while let Some((player, msg)) = mud.connection_broker.poll_player_messages() {
            let mut args_iter = msg.split_whitespace();
            if let Some(cmd) = args_iter.next() {
                let c = mud.commands.iter().find(|c| c.match_name(cmd)).cloned();
                match c {
                    Some(cmd) => (cmd.cmd_fn)(&mut mud, player, &mut args_iter),
                    None => mud
                        .connection_broker
                        .send_player_message(player, get_close_commands(cmd, &mud.commands)),
                };
            }
        }
    }
}

pub fn get_close_commands<'a>(input: &str, commands: &'a Vec<Rc<Command>>) -> String {
    let mut closest = Vec::new();
    for cmd in commands {
        let name_dist = strsim::levenshtein(input, &cmd.name);
        let alias_min = cmd
            .aliases
            .iter()
            .map(|s| (s, strsim::levenshtein(input, &s)))
            .min_by_key(|(_, d)| *d);

        if let Some((alias, alias_dist)) = alias_min {
            if alias_dist < name_dist {
                closest.push((alias.as_str(), alias_dist));
                continue;
            }
        }

        closest.push((cmd.name.as_str(), name_dist));
    }

    closest.sort_by_key(|(_, d)| *d);

    let mut res = format!("Command '{input}' not found, did you mean one of these? ");
    for (cmd, _) in closest.iter().take(3) {
        res.push('\'');
        res.push_str(cmd);
        res.push('\'');
        res.push(' ');
    }

    res
}
