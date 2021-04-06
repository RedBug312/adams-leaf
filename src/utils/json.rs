use std::fs;
use serde::{Deserialize, Serialize};
use crate::network::Network;

#[derive(Serialize, Deserialize)]
struct NetworkJson {
    host_cnt: usize,
    switch_cnt: usize,
    edges: Vec<(usize, usize, f64)>,
}

pub fn load_network(filepath: &str) -> Network {
    let text = fs::read_to_string(filepath)
        .expect("Failed to read network json file");
    let json: NetworkJson = serde_json::from_str(&text)
        .expect("Failed to parse network json file");
    let mut network = Network::default();
    network.add_nodes(json.host_cnt, json.switch_cnt);
    network.add_edges(json.edges);
    network
}
