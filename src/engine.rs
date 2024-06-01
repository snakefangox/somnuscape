use std::{rc::Rc, time::Duration};

use crate::{
    commands::{self, Command},
    connections::{EngineConnectionBroker, PlayerConnectionBroker},
    generation::{GenerationReq, GenerationRes, GeneratorHandle},
    world::{PlaceType, World},
    PlayerEntry, Registry,
};

/// All the engine state kept between ticks, including world info
pub struct Engine {
    pub connection_broker: EngineConnectionBroker,
    pub player_registry: Registry<PlayerEntry>,
    pub gen_handle: GeneratorHandle,
    pub commands: Vec<Rc<Command>>,
    pub world: World,
}

impl Engine {
    pub fn start_engine(
        player_registry: Registry<PlayerEntry>,
        gen_handle: GeneratorHandle,
    ) -> PlayerConnectionBroker {
        let (player_connection_broker, connection_broker) = PlayerConnectionBroker::new();

        std::thread::spawn(move || {
            let mut mud = Engine {
                player_registry,
                connection_broker,
                gen_handle,
                commands: commands::base_commands(),
                world: World::load_or_default(),
            };

            startup_generation(&mut mud);

            run_engine(mud);
        });

        player_connection_broker
    }
}

fn run_engine(mut engine: Engine) -> ! {
    let tick_period = Duration::from_secs_f64(1.0 / 20.0);
    let tick_duration = crossbeam::channel::tick(tick_period);

    loop {
        tick_duration.recv().expect("Tick channel should not close");

        engine.connection_broker.handle_connection_changes();

        while let Some((player, msg)) = engine.connection_broker.poll_player_messages() {
            let mut args_iter = msg.split_whitespace();
            if let Some(cmd) = args_iter.next() {
                let c = engine.commands.iter().find(|c| c.match_name(cmd)).cloned();
                match c {
                    Some(cmd) => (cmd.cmd_fn)(&mut engine, player, &mut args_iter),
                    None => engine
                        .connection_broker
                        .send_player_message(player, get_close_commands(cmd, &engine.commands)),
                };
            }
        }

        incorperate_generation(&mut engine);

        // Try save world every 10 seconds
        engine.world.check_save(10 * 20);
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

fn incorperate_generation(engine: &mut Engine) {
    while let Some(r) = engine.gen_handle.get_responses() {
        match r {
            GenerationRes::Place(places) => places.into_iter().for_each(|p| engine.world.places.push(p)),
        }
    }
}

fn startup_generation(engine: &mut Engine) {
    let villages = engine.world.places.iter().filter(|p| p.place_type == PlaceType::Village).count();
    let dungeons = engine.world.places.iter().filter(|p| p.place_type == PlaceType::Dungeon).count();

    if villages < 3 {
        let count = 3 - villages;
        tracing::info!("Requesting {count} new villages");
        engine.gen_handle.request_generate(GenerationReq::Places(PlaceType::Village, count));
    }

    if dungeons < 5 {
        let count = 5 - dungeons;
        tracing::info!("Requesting {count} new dungeons");
        engine.gen_handle.request_generate(GenerationReq::Places(PlaceType::Dungeon, count));
    }
}
