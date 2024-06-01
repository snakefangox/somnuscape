use std::collections::HashMap;

use anyhow::Result;
use askama::Template;
use petgraph::graph::{NodeIndex, UnGraph};
use rand::seq::IteratorRandom;
use serde::Deserialize;

use crate::{
    filters,
    generation::{extract_md_kv_list, extract_yaml},
    mud::world::{Place, PlaceType, Room},
    AppErrors,
};

use super::AIClient;

pub async fn generate_places(
    client: &AIClient,
    place_type: PlaceType,
    max_count: usize,
) -> Vec<Place> {
    tracing::info!("Generating up to {max_count} {place_type:?}s");

    let mut places = Vec::new();
    loop {
        let place_ideas = generate_place_list(client, &place_type.name())
            .await
            .unwrap_or_default()
            .into_iter()
            .take(max_count - places.len());

        for place_idea in place_ideas {
            match generate_place(client, place_type, &place_idea).await {
                Ok(p) => places.push(p),
                Err(e) => tracing::error!("Couldn't generate {}: {}", place_idea.0, e),
            }
        }

        if places.len() >= max_count {
            break;
        }
    }

    places
}

async fn generate_place(
    client: &AIClient,
    place_type: PlaceType,
    place_idea: &(String, String),
) -> Result<Place> {
    let rooms = generate_rooms(client, place_type, &place_idea).await?;
    let (entrance, rooms, _) = link_rooms(client, place_type, &place_idea.0, rooms).await?;
    Ok(Place::new(place_idea.to_owned(), place_type, entrance, rooms.into()))
}

#[derive(Template, Default)]
#[template(path = "place_list.md")]
struct CompletionTemplate<'a> {
    place_type: &'a str,
}

async fn generate_place_list(client: &AIClient, place_type: &str) -> Result<Vec<(String, String)>> {
    let res = client
        .generate(CompletionTemplate { place_type }.to_string())
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
    place_type: PlaceType,
    place: &(String, String),
) -> Result<Vec<Room>> {
    tracing::info!("Generating rooms for {}", place.0);
    let rooms = extract_md_kv_list(
        &client
            .generate(
                GenerateRoomsTemplate {
                    place_type: &place_type.name(),
                    place_name: &place.0,
                    place_description: &place.1,
                    room_type: &place_type.room_type(),
                }
                .to_string(),
            )
            .await?,
    )
    .into_iter()
    .map(|(n, d)| Room::new(n, d))
    .collect();

    Ok(rooms)
}

#[derive(Template)]
#[template(path = "link_rooms.md")]
struct LinkRoomsTemplate<'a> {
    place_type: PlaceType,
    place_name: &'a str,
    rooms: &'a [Room],
}

#[derive(Debug, Deserialize)]
struct LinkRoomsOutput {
    entrance: String,
    connections: HashMap<String, Vec<String>>,
}

async fn link_rooms(
    client: &AIClient,
    place_type: PlaceType,
    place_name: &str,
    rooms: Vec<Room>,
) -> Result<(NodeIndex, UnGraph<Room, ()>, HashMap<String, NodeIndex>)> {
    tracing::info!("Linking rooms for {place_name}");
    let links: LinkRoomsOutput = extract_yaml(
        &client
            .generate(
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

    let mut graph = UnGraph::new_undirected();
    let name_to_node_idx: HashMap<_, _> = rooms
        .into_iter()
        .map(|r| (r.name().to_string(), graph.add_node(r)))
        .collect();

    for (node, connections) in &links.connections {
        if let Some(a) = name_to_node_idx.get(node) {
            for con in connections {
                if let Some(b) = name_to_node_idx.get(con) {
                    if !graph.contains_edge(*a, *b) {
                        graph.add_edge(*a, *b, ());
                    }
                }
            }
        }
    }

    let entrance = links.entrance.trim().to_string();
    let entrance_idx = *name_to_node_idx
        .get(&entrance)
        .ok_or(AppErrors::AIStructureError)?;

    for node in graph.node_indices() {
        if !petgraph::algo::has_path_connecting(&graph, entrance_idx, node, None) {
            graph.add_edge(
                graph
                    .node_indices()
                    .choose(&mut rand::thread_rng())
                    .unwrap(),
                node,
                (),
            );
        }
    }

    Ok((entrance_idx, graph, name_to_node_idx))
}
