use std::collections::HashMap;

use anyhow::Result;
use askama::Template;
use futures::{stream, Stream, StreamExt};
use serde::Deserialize;

use crate::{
    filters,
    generation::{extract_md_kv_list, extract_yaml},
    mud::world::{Direction, Location, Place},
    AppErrors,
};

use super::AIClient;

pub async fn generate_places<'a>(
    client: &'a AIClient,
    place_type: &'a PlaceType,
    max_count: usize,
) -> impl Stream<Item = (Place, HashMap<Location, Place>)> + 'a {
    tracing::info!("Generating up to {} {}s", max_count, place_type.name);

    let idea_stream = stream::unfold(
        (max_count, Vec::new()),
        move |(remaining_count, mut place_ideas): (usize, Vec<(String, String)>)| async move {
            if remaining_count == 0 {
                return None;
            }

            let remaining_ungenerated = remaining_count.saturating_sub(place_ideas.len());

            while place_ideas.is_empty() && remaining_ungenerated > 0 {
                let places = generate_place_list(client, &place_type.name, remaining_ungenerated)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .take(remaining_ungenerated);

                place_ideas.extend(places);
            }

            place_ideas
                .pop()
                .map(|i| (i, (remaining_count.saturating_sub(1), place_ideas)))
        },
    );

    let place_stream = idea_stream
        .map(move |place_idea| async move {
            let mut place = generate_place(client, place_type, &place_idea).await;
            while let Err(e) = place {
                tracing::error!("Failed to generate place: {e}");
                place = generate_place(client, place_type, &place_idea).await;
            }

            place.unwrap()
        })
        // Generate a few at once
        .buffered(3);

    place_stream
}

async fn generate_place(
    client: &AIClient,
    place_type: &PlaceType,
    place_idea: &(String, String),
) -> Result<(Place, HashMap<Location, Place>)> {
    let rooms = generate_rooms(client, place_type, &place_idea).await?;
    let mut overworld_place = Place::new(
        format!("Overworld - {}", place_idea.0),
        place_idea.1.to_owned(),
    );
    let (entrance, mut rooms) = link_rooms(client, place_type, &place_idea.0, rooms).await?;

    overworld_place.add_connection(Direction::Down, entrance)?;
    rooms
        .get_mut(&entrance)
        .unwrap()
        .add_connection(Direction::Up, overworld_place.location)?;

    Ok((overworld_place, rooms))
}

#[derive(Template, Default)]
#[template(path = "place_list.md")]
struct CompletionTemplate<'a> {
    place_type: &'a str,
    count: usize,
}

async fn generate_place_list(
    client: &AIClient,
    place_type: &str,
    count: usize,
) -> Result<Vec<(String, String)>> {
    let res = client
        .generate_with_tone(CompletionTemplate { place_type, count }.to_string())
        .await?;

    Ok(extract_md_kv_list(&res))
}

#[derive(Template, Default)]
#[template(path = "generate_rooms.md")]
struct GenerateRoomsTemplate<'a> {
    place_type: &'a str,
    room_type: &'a str,
    place_name: &'a str,
    place_description: &'a str,
}

async fn generate_rooms(
    client: &AIClient,
    place_type: &PlaceType,
    place: &(String, String),
) -> Result<Vec<Place>> {
    tracing::info!("Generating rooms for {}", place.0);
    let rooms = extract_md_kv_list(
        &client
            .generate_with_tone(
                GenerateRoomsTemplate {
                    place_type: &place_type.name,
                    place_name: &place.0,
                    place_description: &place.1,
                    room_type: &place_type.room_type,
                }
                .to_string(),
            )
            .await?,
    )
    .into_iter()
    .map(|(n, d)| Place::new(n, d))
    .collect();

    Ok(rooms)
}

#[derive(Template)]
#[template(path = "link_rooms.md")]
struct LinkRoomsTemplate<'a> {
    place_type: &'a PlaceType,
    place_name: &'a str,
    rooms: &'a [Place],
}

#[derive(Debug, Deserialize)]
struct LinkRoomsOutput {
    entrance: String,
    connections: HashMap<String, Vec<String>>,
}

async fn link_rooms(
    client: &AIClient,
    place_type: &PlaceType,
    place_name: &str,
    rooms: Vec<Place>,
) -> Result<(Location, HashMap<Location, Place>)> {
    tracing::info!("Linking rooms for {place_name}");

    let links: LinkRoomsOutput = extract_yaml(
        &client
            .generate_simple(
                LinkRoomsTemplate {
                    place_type,
                    place_name,
                    rooms: &rooms,
                }
                .to_string(),
            )
            .await?,
    )
    .map_err(|_| AppErrors::AIStructureError)?;

    let mut rooms: HashMap<_, _> = rooms.into_iter().map(|r| (r.location, r)).collect();
    let name_to_location: HashMap<_, _> = rooms
        .iter()
        .map(|(l, r)| (r.name.to_string(), *l))
        .collect();

    for (node, connections) in &links.connections {
        if let Some(a) = name_to_location.get(node) {
            for con in connections {
                if let Some(b) = name_to_location.get(con) {
                    if !rooms[a].is_connected(*b) {
                        let dir = rooms
                            .get_mut(a)
                            .unwrap()
                            .add_connection(Direction::North, *b)
                            .map_err(|_| AppErrors::AIStructureError)?;

                        rooms
                            .get_mut(b)
                            .unwrap()
                            .add_connection(dir.reverse(), *a)
                            .map_err(|_| AppErrors::AIStructureError)?;
                    }
                }
            }
        }
    }

    let entrance = links.entrance.trim().to_string();
    let entrance_idx = *name_to_location
        .get(&entrance)
        .ok_or(AppErrors::AIStructureError)?;

    Ok((entrance_idx, rooms))
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlaceType {
    name: &'static str,
    room_type: &'static str,
    room_types_pural: &'static str,
}

pub const DUNGEON_PLACE_TYPE: PlaceType = PlaceType {
    name: "dungeon",
    room_type: "room or corridor",
    room_types_pural: "rooms or corridors",
};
pub const VILLAGE_PLACE_TYPE: PlaceType = PlaceType {
    name: "village",
    room_type: "building or street",
    room_types_pural: "buildings or streets",
};
