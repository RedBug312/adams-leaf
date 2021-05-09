use std::fs;

use serde::Deserialize;

use super::config::Config;
use super::stream::{AVB, TSN};
use crate::network::Network;

#[derive(Deserialize)]
struct NetworkYaml {
    scale: NetworkScaleYaml,
    edges: Vec<NetworkEdgeYaml>,
}

#[derive(Deserialize)]
struct NetworkScaleYaml {
    end_devices: usize,
    bridges: usize,
}

#[derive(Deserialize)]
struct NetworkEdgeYaml {
    ends: [usize; 2],
    bandwidth: f64,
}

#[derive(Deserialize)]
struct StreamsYaml {
    scale: StreamsScaleYaml,
    tsns: Vec<TSN>,
    avbs: Vec<AVB>,
}

#[derive(Deserialize)]
struct StreamsScaleYaml {
    tsns: usize,
    avbs: usize,
    hyperperiod: u32,
    end_devices: usize,
}

pub fn load_network(path: &str) -> Network {
    let text = fs::read_to_string(path)
        .expect("Failed to read network yaml file");
    let yaml: NetworkYaml = serde_yaml::from_str(&text)
        .expect("Failed to parse network yaml file");
    let mut network = Network::default();
    let switches = yaml.scale.end_devices + yaml.scale.bridges;
    debug_assert_eq!(switches, check_switches(&yaml));
    network.add_nodes(yaml.scale.end_devices, yaml.scale.bridges);
    network.add_edges(flatten(yaml.edges));
    network
}

pub fn load_streams(path: &str, fold: u32) -> (Vec<TSN>, Vec<AVB>) {
    let text = fs::read_to_string(path)
        .expect("Failed to read streams yaml file");
    let yaml: StreamsYaml = serde_yaml::from_str(&text)
        .expect("Failed to parse streams yaml file");
    debug_assert_eq!(yaml.scale.tsns, yaml.tsns.len());
    debug_assert_eq!(yaml.scale.avbs, yaml.avbs.len());
    debug_assert_eq!(yaml.scale.hyperperiod, check_hyperperiod(&yaml));
    debug_assert_eq!(yaml.scale.end_devices, check_end_devices(&yaml));
    (repeated(yaml.tsns, fold), repeated(yaml.avbs, fold))
}

pub fn load_config(path: &str) -> Config {
    let text = fs::read_to_string(path)
        .expect("Failed to read config yaml file");
    #[allow(clippy::let_and_return)]
    let yaml = serde_yaml::from_str(&text)
        .expect("Failed to parse config yaml file");
    yaml
}

fn check_switches(yaml: &NetworkYaml) -> usize {
    let ends = yaml.edges.iter().map(|e| e.ends[0].max(e.ends[1]));
    ends.fold(0, usize::max) + 1
}

fn check_hyperperiod(yaml: &StreamsYaml) -> u32 {
    let tsns_period = yaml.tsns.iter().map(|s| s.period);
    let avbs_period = yaml.avbs.iter().map(|s| s.period);
    tsns_period.chain(avbs_period).fold(1, num::integer::lcm)
}

fn check_end_devices(yaml: &StreamsYaml) -> usize {
    let tsns_ends = yaml.tsns.iter().map(|s| s.src.max(s.dst));
    let avbs_ends = yaml.avbs.iter().map(|s| s.src.max(s.dst));
    tsns_ends.chain(avbs_ends).fold(0, usize::max) + 1
}

fn flatten(edges: Vec<NetworkEdgeYaml>) -> Vec<(usize, usize, f64)> {
    edges.into_iter()
        .map(|e| (e.ends[0], e.ends[1], e.bandwidth))
        .collect()
}

fn repeated<T: Clone>(vec: Vec<T>, mul: u32) -> Vec<T> {
    // taken from stackoverflow.com/a/28437687
    let length = vec.len() * mul as usize;
    vec.iter().cloned().cycle().take(length).collect()
}
