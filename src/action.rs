use crate::{dungeon::Dungeon, player::Player, web_types::State};
use askama::Template;
use futures::future::BoxFuture;
use lazy_static::lazy_static;

pub enum ActionInput {
    Button(String),
}

pub struct Action {
    pub name: &'static str,
    pub can_use: fn(&Player) -> bool,
    pub get_inputs: fn(&Player, &State) -> BoxFuture<'static, Vec<ActionInput>>,
}

lazy_static! {
    static ref ACTIONS: Vec<Action> = create_actions();
}

#[derive(Template)]
#[template(path = "elements/action_menu.html")]
struct ActionMenu<'a> {
    names: Vec<&'a str>,
}

#[derive(Template)]
#[template(path = "elements/action_input.html")]
pub struct ActionInputMenu<'a> {
    inputs: &'a Vec<ActionInput>,
}

pub fn get_active_actions(player: &Player) -> String {
    ActionMenu {
        names: ACTIONS
            .iter()
            .filter(|a| (a.can_use)(player))
            .map(|a| a.name)
            .collect(),
    }
    .render()
    .unwrap()
}

pub async fn get_input_menu(player: &Player, state: &State, action_name: &String) -> String {
    let action = ACTIONS.iter().find(|a| a.name == action_name);
    if action.is_none() {
        return String::new();
    }

    let inputs = (action.unwrap().get_inputs)(player, state).await;

    ActionInputMenu { inputs: &inputs }.render().unwrap()
}

fn create_actions() -> Vec<Action> {
    vec![Action {
        name: "Embark",
        can_use: |p| p.location.is_town(),
        get_inputs: |_, s| {
            let s = s.clone();

            Box::pin(async move {
                s.list::<Dungeon>()
                    .await
                    .iter()
                    .cloned()
                    .map(|n| ActionInput::Button(n))
                    .collect()
            })
        },
    }]
}
