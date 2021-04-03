use std::fs;
use serde::{Deserialize, Serialize};
use crate::network::Network;
use crate::utils::stream::{TSN, AVB};

use super::config::Config;


#[derive(Serialize, Deserialize)]
struct StreamsJson {
    tt_flows: Vec<TSN>,
    avb_flows: Vec<AVB>,
}

#[derive(Serialize, Deserialize)]
struct NetworkJson {
    host_cnt: usize,
    switch_cnt: usize,
    edges: Vec<(usize, usize, f64)>,
}


pub fn load_streams(filepath: &str, times: u32) -> (Vec<TSN>, Vec<AVB>) {
    let text = fs::read_to_string(filepath)
        .expect("Failed to read streams json file");
    let json: StreamsJson = serde_json::from_str(&text)
        .expect("Failed to parse streams json file");
    let tsns = json.tt_flows;
    let avbs = json.avb_flows;
    (repeated(tsns, times), repeated(avbs, times))
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

pub fn load_config(filepath: &str) -> Config {
    let text = fs::read_to_string(filepath)
        .expect("Failed to read network json file");
    let json: Config = serde_json::from_str(&text)
        .expect("Failed to parse network json file");
    json
}

fn repeated<T: Clone>(vec: Vec<T>, mul: u32) -> Vec<T> {
    // taken from stackoverflow.com/a/28437687
    let length = vec.len() * mul as usize;
    vec.iter().cloned().cycle().take(length).collect()
}
