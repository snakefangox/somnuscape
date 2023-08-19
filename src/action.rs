use crate::{core::Location, dungeon::{Dungeon, Direction}, player::Player, web_types::State};
use askama::Template;

pub enum ActionInput {
    Button(String),
}

pub enum Action {
    // Town
    Embark,
    // Dungeon
    Move,
    Return,
}

#[derive(Template)]
#[template(path = "elements/action_menu.html")]
struct ActionMenu<'a> {
    names: Vec<&'a str>,
}

#[derive(Template)]
#[template(path = "elements/action_input.html")]
pub struct ActionInputMenu<'a> {
    action_name: &'a str,
    inputs: &'a Vec<ActionInput>,
}

impl Action {
    pub fn name(&self) -> &'static str {
        match self {
            Action::Embark => "Embark",
            Action::Move => "Move",
            Action::Return => "Return",
        }
    }

    pub fn actions() -> &'static [Action] {
        static ACTIONS: [Action; 3] = [Action::Embark, Action::Move, Action::Return];
        &ACTIONS
    }

    pub fn currently_valid(&self, player: &Player, dungeon: Option<&Dungeon>) -> bool {
        match self {
            Action::Embark => player.location.is_town(),
            Action::Move => player.location.is_dungeon(),
            Action::Return => player.location.is_dungeon() && dungeon.unwrap().rooms.first().unwrap().name == player.location.room(),
        }
    }

    pub async fn input_menu(&self, player: &Player, state: &State) -> Vec<ActionInput> {
        match self {
            Action::Embark => state
                .list::<Dungeon>()
                .await
                .iter()
                .cloned()
                .map(|n| ActionInput::Button(n))
                .collect(),
            Action::Move => state
                .get::<Dungeon>(&player.location.area())
                .await
                .unwrap()
                .room(&player.location.room())
                .unwrap()
                .connections
                .iter()
                .map(|c| ActionInput::Button(format!("{:?}: {}", c.0, c.1)))
                .collect(),
            Action::Return => vec![ActionInput::Button("Return".to_owned())],
        }
    }

    pub async fn perform_action(&self, player: &mut Player, state: &State, option_name: &str) {
        match self {
            Action::Embark => {
                if let Some(dungeon) = state.get::<Dungeon>(option_name).await {
                    player.location = Location::Dungeon {
                        area: option_name.to_owned(),
                        room: dungeon.rooms.first().unwrap().name.to_owned(),
                    };
                }
            }
            Action::Move => {
                let dungeon = state.grab::<Dungeon>(&&player.location.area()).await;
                let room = dungeon.room(&player.location.room()).unwrap();
                let dir = option_name.split(":").next().map(Direction::from_str).unwrap_or_default();
                if room.connections.contains_key(&dir) {
                    player.location.move_room(&room.connections[&dir]);
                }
            }
            Action::Return => {
                player.location = Location::Town;
            }
        }
    }
}

pub async fn get_active_actions(player: &Player, state: &State) -> String {
    let dungeon_op = state.get::<Dungeon>(&player.location.area()).await;
    let dungeon = dungeon_op.as_ref();
    ActionMenu {
        names: Action::actions()
            .iter()
            .filter(|a| a.currently_valid(player, dungeon))
            .map(|a| a.name())
            .collect(),
    }
    .render()
    .unwrap()
}

pub async fn get_input_menu(player: &Player, state: &State, action_name: &str) -> String {
    let action = Action::actions().iter().find(|a| a.name() == action_name);
    if action.is_none() {
        return String::new();
    }

    let action = action.unwrap();
    let inputs = action.input_menu(player, state).await;

    ActionInputMenu {
        action_name: action.name(),
        inputs: &inputs,
    }
    .render()
    .unwrap()
}

pub async fn perform_action(
    player: &mut Player,
    state: &State,
    action_name: &str,
    option_name: &str,
) -> String {
    let action = Action::actions().iter().find(|a| a.name() == action_name);
    if action.is_none() {
        return get_active_actions(player, state).await;
    }

    let action = action.unwrap();
    action.perform_action(player, state, option_name).await;
    state.set(player).await;

    get_active_actions(player, state).await
}
