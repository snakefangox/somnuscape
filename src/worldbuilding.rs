use std::{collections::HashSet, time::Duration};

use rand::seq::{SliceRandom, IteratorRandom};

use crate::{
    core::Conversation,
    dungeon::{Creature, Dungeon, DungeonGenerator, DungeonLevel, DungeonSize},
    web_types::State,
};

const STORYTELLER_INTERVAL: Duration = Duration::from_secs(30);

pub async fn run() {
    let state = State::new();

    let mut interval = tokio::time::interval(STORYTELLER_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        let dungeons: HashSet<String> = state.list::<Dungeon>().await;
        if dungeons.len() < 3 {
            let _ = generate_dungeons(&state, 3).await;
        }

        interval.tick().await;
    }
}

async fn generate_dungeons(state: &State, n: usize) -> anyhow::Result<()> {
    let names = generate_names(n, "dungeon").await?;

    for name in names {
        let (dungeon, creatures) = DungeonGenerator(
            name,
            DungeonLevel::Low,
            DungeonSize::Medium,
        )
        .generate()
        .await?;

        for creature in creatures {
            let _ = get_creature(state, &creature).await;
        }
        state.set(&dungeon).await;
    }

    Ok(())
}

async fn generate_names(n: usize, name_type: &str) -> anyhow::Result<Vec<String>> {
    const SEED_WORDS: [&'static str; 18] = [
        "dragon", "elf", "dwarf", "undead", "astral", "eye", "eldritch", "sanity", "boiling",
        "ocean", "holy", "cursed", "ancient", "frozen", "gate", "hold", "spirit", "temporal",
    ];

    let mut names = Vec::new();
    let mut name_conv = Conversation::prime(include_str!("../primers/names.yaml"));
    name_conv.temprature(1.0);

    let mut e = SEED_WORDS.choose_multiple(&mut rand::thread_rng(), 3);
    let seed = format!(
        "{:?} {:?} {:?}",
        e.next().unwrap(),
        e.next().unwrap(),
        e.next().unwrap()
    );

    for _ in 0..(n / 3) + 1 {
        let raw_names = name_conv
            .query(&format!("seed: {seed}\nFive fantasy {} names", name_type))
            .await?
            .1;

        let r = regex::Regex::new(r"(?m)^[0-9]+\. (.*)$")?;
        let gen_names: Vec<&str> = r
            .captures_iter(&raw_names)
            .filter_map(|c| c.get(1).map(|c| c.as_str()))
            .choose_multiple(&mut rand::thread_rng(), 3);

        names.extend(gen_names.iter().map(|s| s.to_string()));
    }

    Ok(names)
}

pub async fn get_creature(state: &State, name: &str) -> anyhow::Result<Creature> {
    if state.has::<Creature>(name).await {
        Ok(state.grab(name).await)
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

#[tokio::test]
async fn test_names() {
    dotenvy::dotenv().unwrap();
    let names = generate_names(10, "dungeons").await.unwrap();

    println!("{:#?}", names)
}
