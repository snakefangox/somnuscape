use std::{time::Duration, collections::HashSet};

use crate::{web_types::State, dungeon::{Dungeon, Creature}, core::Conversation};


const STORYTELLER_INTERVAL: Duration = Duration::from_secs(30);

pub async fn run() {
    let state = State::new();

    let mut interval = tokio::time::interval(STORYTELLER_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        let dungeons: HashSet<String> = state.list::<Dungeon>().await;
        if dungeons.len() < 10 {
            let _ = generate_dungeons(&state).await;
        }

        interval.tick().await;
    }
}

async fn generate_dungeons(state: &State) -> anyhow::Result<()> {
    let mut dungeon_conv = Conversation::prime(include_str!("../primers/rooms.yaml"));
    let names = generate_names(10, "dungeon").await?;

    for name in names {
        let json = dungeon_conv
            .query(&format!("dungeon_name: {}", name))
            .await?
            .1;

        let (dungeon, creatures) = Dungeon::from_json(&json)?;
        for creature in creatures {
            let _ = get_creature(state, &creature).await;
        }
        state.set(&dungeon).await;
    }

    Ok(())
}

async fn generate_names(n: usize, name_type: &str) -> anyhow::Result<Vec<String>> {
    const SLACK: usize = 5;

    let mut name_conv = Conversation::prime(include_str!("../primers/names.yaml"));
    name_conv
        .say(&format!("{} fantasy {} names", n + SLACK, name_type))
        .await?;

    let best_names = name_conv
        .say("Order them from most interesting and unique to least interesting and unique")
        .await?
        .1;

    let r = regex::Regex::new(r"(?m)^[0-9]+\. (.*)$")?;
    let names: Vec<String> = r
        .captures_iter(&best_names)
        .filter_map(|c| c.get(1).map(|c| c.as_str().to_owned()))
        .take(n)
        .collect();
    Ok(names)
}

pub async fn get_creature(state: &State, name: &str) -> anyhow::Result<Creature> {
    if state.has::<Creature>(name).await {
        Ok(state.get(name).await.unwrap())
    } else {
        let result = serde_json::from_str::<Creature>(
            &Conversation::prime(include_str!("../primers/stats.yaml"))
                .query(&format!("creature_name: {}", name))
                .await?
                .1,
        )?;
        state.set(&result).await;

        Ok(result)
    }
}