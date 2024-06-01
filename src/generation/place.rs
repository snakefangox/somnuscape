use std::collections::HashMap;

use anyhow::Result;
use askama::Template;
use futures::{future, FutureExt};
use petgraph::graph::{NodeIndex, UnGraph};
use rand::seq::IteratorRandom;
use serde::Deserialize;

use crate::{
    filters, generation::{extract_md_kv_list, extract_yaml, generate}, world::{Place, PlaceType, Room}, AppErrors
};

pub async fn generate_places(place_type: PlaceType, max_count: usize) -> Vec<Place> {
    tracing::info!("Generating up to {max_count} {place_type:?}s");
    let place_ideas = generate_place_list(&place_type.name())
        .await
        .unwrap_or_default();

    let room_futures = place_ideas.into_iter().take(max_count).map(|p| {
        async move {
            (
                generate_rooms(&place_type.name(), &place_type.room_type(), p.clone()).await,
                p,
            )
        }
        .then(|(r, p)| async_linker_block(r, place_type, p))
    });

    future::join_all(room_futures)
        .await
        .into_iter()
        .filter_map(|(r, p)| {
            if let Ok(r) = r {
                Some(Place::new(p, place_type, r.0, r.1.into()))
            } else {
                None
            }
        })
        .collect()
}

async fn async_linker_block(
    r: Result<Vec<Room>>,
    place_type: PlaceType,
    p: (String, String),
) -> (
    Result<(NodeIndex, UnGraph<Room, ()>, HashMap<String, NodeIndex>)>,
    (String, String),
) {
    if let Ok(r) = r {
        (
            link_rooms(&place_type.name(), &place_type.room_type(), &p.0, r).await,
            p,
        )
    } else {
        (Err(r.unwrap_err()), p)
    }
}

#[derive(Template, Default)]
#[template(path = "place_list.md")]
struct CompletionTemplate<'a> {
    place_type: &'a str,
} 

async fn generate_place_list(place_type: &str) -> Result<Vec<(String, String)>> {
    let res = generate(CompletionTemplate { place_type }.to_string()).await?;

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
    place_type: &str,
    room_type: &str,
    place: (String, String),
) -> Result<Vec<Room>> {
    tracing::info!("Generating rooms for {}", place.0);
    let rooms = extract_md_kv_list(
        &generate(
            GenerateRoomsTemplate {
                place_type,
                place_name: &place.0,
                place_description: &place.1,
                room_type,
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

#[derive(Template, Default)]
#[template(path = "link_rooms.md")]
struct LinkRoomsTemplate<'a> {
    place_type: &'a str,
    place_name: &'a str,
    room_type: &'a str,
    rooms: &'a [Room],
}

#[derive(Debug, Deserialize)]
struct LinkRoomsOutput {
    entrance: String,
    connections: HashMap<String, Vec<String>>,
}

async fn link_rooms(
    place_type: &str,
    room_type: &str,
    place_name: &str,
    rooms: Vec<Room>,
) -> Result<(NodeIndex, UnGraph<Room, ()>, HashMap<String, NodeIndex>)> {
    tracing::info!("Linking rooms for {place_name}");
    let links: LinkRoomsOutput = extract_yaml(
        &generate(
            LinkRoomsTemplate {
                place_type,
                place_name,
                room_type,
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
