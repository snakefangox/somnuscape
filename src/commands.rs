use std::rc::Rc;

use crate::{engine::Engine, mud::world::Direction};

pub type CmdFn = Box<dyn Fn(&mut Engine, usize, &mut dyn Iterator<Item = &str>)>;

pub struct Command {
    pub name: String,
    pub aliases: Vec<String>,
    pub help: String,
    pub cmd_fn: CmdFn,
}

impl Command {
    pub fn new(name: &str, aliases: &[&str], help: &str, cmd_fn: CmdFn) -> Self {
        Self {
            name: name.to_string(),
            aliases: aliases.iter().map(|s| s.to_string()).collect(),
            help: help.to_string(),
            cmd_fn,
        }
    }

    pub fn match_name(&self, name: &str) -> bool {
        self.name == name || self.aliases.iter().any(|a| a == name)
    }
}

pub fn base_commands() -> Vec<Rc<Command>> {
    let mut base = vec![
        help_command().into(),
        quit_command().into(),
        look_command().into(),
    ];
    base.extend(move_commands());
    base
}

pub fn look_command() -> Command {
    Command::new(
        "look",
        &["l"],
        "",
        Box::new(|engine, player, _| {
            let player_character = engine.world.player_characters.entry(player).or_default();

            if let Some(place) = engine.world.places.get(&player_character.location) {
                let look_msg = place.look(&engine.world, "You're standing in");

                engine
                    .connection_broker
                    .send_player_message(player, look_msg);
            } else {
                engine.connection_broker.send_player_message(
                    player,
                    "Invalid location, resetting to start".to_string(),
                );
                player_character.location = engine
                    .world
                    .overworld_locales
                    .first()
                    .map(|l| *l)
                    .unwrap_or_default();
            }
        }),
    )
}

pub fn move_commands() -> Vec<Rc<Command>> {
    let mut move_commands = Vec::new();

    for direction in Direction::values() {
        let cmd = Command::new(
            &direction.name(),
            &[&direction.name()[0..1]],
            "",
            Box::new(move |engine, player, _| {
                let player_character = engine.world.player_characters.entry(player).or_default();

                if let Some(place) = engine.world.places.get(&player_character.location) {
                    match place.connections().get(&direction) {
                        Some(l) => {
                            player_character.location = *l;
                            let new_place = &engine.world.places[l];
                            engine.connection_broker.send_player_message(
                                player,
                                new_place.look(&engine.world, "You move to"),
                            );
                        }
                        None => {
                            engine.connection_broker.send_player_message(
                                player,
                                format!("You cannot go {direction:?} from here"),
                            );
                        }
                    }
                } else {
                    engine.connection_broker.send_player_message(
                        player,
                        "Invalid location, resetting to start".to_string(),
                    );
                    player_character.location = engine
                        .world
                        .overworld_locales
                        .first()
                        .map(|l| *l)
                        .unwrap_or_default();
                }
            }),
        );

        move_commands.push(cmd.into());
    }

    move_commands
}

pub fn quit_command() -> Command {
    Command::new(
        "quit",
        &["exit"],
        "",
        Box::new(|engine, player, _| {
            let player_reg = engine.player_registry.blocking_read();
            let name = player_reg
                .get(player)
                .map(|p| p.username.as_str())
                .unwrap_or("");

            engine
                .connection_broker
                .send_player_message(player, format!("Logging out, goodbye {name}!"));
            engine.connection_broker.disconnect_player(player);
        }),
    )
}

pub fn help_command() -> Command {
    Command::new(
        "help",
        &["?"],
        "",
        Box::new(|engine, player, args| {
            let res = match args.next() {
                Some(cmd) => {
                    let cmd_help = engine.commands.iter().find(|c| c.match_name(cmd));

                    if let Some(cmd_help) = cmd_help {
                        let mut res = String::new();
                        res.push_str("Command: ");
                        res.push_str(&cmd_help.name);
                        res.push('\n');

                        if !cmd_help.aliases.is_empty() {
                            res.push_str("Aliases: ");
                            for alias in &cmd_help.aliases {
                                res.push_str(alias);
                                res.push(' ');
                            }
                            res.push('\n');
                        }

                        res.push('\n');
                        res.push_str(&cmd_help.help);
                        res
                    } else {
                        format!("Command provided: {cmd} does not exist, try running just 'help' to list commands")
                    }
                }
                None => {
                    let mut res = String::new();
                    res.push_str("Listing all commands\nRun 'help <command name>' to get help for a specific command\n\n");
                    let mut count = 0;
                    for cmd in &engine.commands {
                        res.push_str(&format!("{:20}", cmd.name));
                        count += 1;
                        if count % 4 == 0 {
                            res.push('\n');
                        }
                    }
                    res
                }
            };

            engine.connection_broker.send_player_message(player, res);
        }),
    )
}
