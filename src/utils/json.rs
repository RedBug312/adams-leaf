use std::fs;
use serde::{Deserialize, Serialize};
use crate::network::Network;
use crate::utils::stream::{TSN, AVB};


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
    let mut graph = Network::default();
    graph.add_nodes(json.host_cnt, json.switch_cnt);
    graph.add_edges(json.edges);
    graph
}

fn repeated<T: Clone>(vec: Vec<T>, mul: u32) -> Vec<T> {
    // taken from stackoverflow.com/a/28437687
    let length = vec.len() * mul as usize;
    vec.iter().cloned().cycle().take(length).collect()
}


#[cfg(test)]
mod test {
    use super::repeated;
    #[test]
    fn test_repeated() {
        let vec = vec!["vector", "of", "string"];
        assert_eq!(repeated(vec.clone(), 0).len(), 0);
        assert_eq!(repeated(vec.clone(), 1).len(), 3);
        assert_eq!(repeated(vec.clone(), 3).len(), 9);
    }
}
