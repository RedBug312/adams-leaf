use std::fs;
use super::{config::Config, stream::{AVB, TSN}};
use serde::Deserialize;

#[derive(Deserialize)]
struct StreamsYaml {
    pub name: String,
    pub scale: StreamsScaleYaml,
    pub tsns: Vec<TSN>,
    pub avbs: Vec<AVB>,
}

#[derive(Deserialize)]
struct StreamsScaleYaml {
    pub tsns: usize,
    pub avbs: usize,
    pub hyperperiod: u32,
    pub switches: usize,
}

pub fn load_streams(path: &str, fold: u32) -> (Vec<TSN>, Vec<AVB>) {
    let text = fs::read_to_string(path)
        .expect("Failed to read network yaml file");
    let yaml: StreamsYaml = serde_yaml::from_str(&text)
        .expect("Failed to parse network yaml file");
    debug_assert_eq!(yaml.scale.tsns, yaml.tsns.len());
    debug_assert_eq!(yaml.scale.avbs, yaml.avbs.len());
    debug_assert_eq!(yaml.scale.hyperperiod, check_hyperperiod(&yaml));
    (repeated(yaml.tsns, fold), repeated(yaml.avbs, fold))
}

pub fn load_config(path: &str) -> Config {
    let text = fs::read_to_string(path)
        .expect("Failed to read network yaml file");
    let yaml = serde_yaml::from_str(&text)
        .expect("Failed to parse network yaml file");
    yaml
}

fn check_hyperperiod(yaml: &StreamsYaml) -> u32 {
    let tsns_period = yaml.tsns.iter().map(|s| s.period);
    let avbs_period = yaml.avbs.iter().map(|s| s.period);
    tsns_period.chain(avbs_period).fold(1, num::integer::lcm)
}

fn repeated<T: Clone>(vec: Vec<T>, mul: u32) -> Vec<T> {
    // taken from stackoverflow.com/a/28437687
    let length = vec.len() * mul as usize;
    vec.iter().cloned().cycle().take(length).collect()
}
