use std::{collections::HashMap, rc::Rc, time::Duration};

use crate::{
    commands::{self, Command},
    config,
    connections::{EngineConnectionBroker, PlayerConnectionBroker},
    generation::{GenerationReq, GenerationRes, GeneratorHandle},
    mud::world::{Direction, Location, Place, World},
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
    let tick_period = Duration::from_secs_f64(1.0 / config::get().ticks_per_second);
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

        engine.world.check_save(config::get().save_every_x_ticks);
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
            GenerationRes::Place(places) => places
                .into_iter()
                .for_each(|(place, rooms)| add_new_locale(engine, place, rooms)),
        }
    }
}

/// Add a new overworld map entry to the world and connect it to existing entries
fn add_new_locale(engine: &mut Engine, mut place: Place, rooms: HashMap<Location, Place>) {
    for ow_location in &engine.world.overworld_locales {
        let ow_place = engine.world.places.get_mut(ow_location).unwrap();
        // Limit to 5 connections to avoid adding things in the up direction
        if ow_place.connections().len() < 5 {
            let dir = ow_place
                .add_connection(Direction::North, place.location)
                .expect("Should be able to add overworld connection");
            place
                .add_connection(dir.reverse(), *ow_location)
                .expect("Should be able to add overworld connection");
            break;
        }
    }

    engine.world.overworld_locales.push(place.location);
    engine.world.places.insert(place.location, place);

    for (location, room) in rooms.into_iter() {
        engine.world.places.insert(location, room);
    }
}

fn startup_generation(engine: &mut Engine) {
    if engine.world.places.len() == 0 {
        let count = 3;
        tracing::info!("Requesting {count} new villages");
        engine.gen_handle.request_generate(GenerationReq::Places(
            crate::generation::VILLAGE_PLACE_TYPE,
            count,
        ));

        let count = 5;
        tracing::info!("Requesting {count} new dungeons");
        engine.gen_handle.request_generate(GenerationReq::Places(
            crate::generation::DUNGEON_PLACE_TYPE,
            count,
        ));
    }
}
