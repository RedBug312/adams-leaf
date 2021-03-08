use serde::{Deserialize, Serialize};
use std::fs;
use crate::network::Network;
use crate::utils::stream::{Flow, AVBFlow, TSNFlow};
use crate::utils::stream::data::{TSNData, AVBData, AVBClass};

pub fn read_flows_from_file(file_name: &str, times: usize) -> (Vec<TSNFlow>, Vec<AVBFlow>) {
    let mut tsns = Vec::<TSNFlow>::new();
    let mut avbs = Vec::<AVBFlow>::new();
    for _ in 0..times {
        read_flows_from_file_once(&mut tsns, &mut avbs, file_name);
    }
    (tsns, avbs)
}
fn read_flows_from_file_once(tsns: &mut Vec<TSNFlow>, avbs: &mut Vec<AVBFlow>, file_name: &str) {
    let txt = fs::read_to_string(file_name).expect(&format!("找不到檔案: {}", file_name));
    let all_flows: AllFlows =
        serde_json::from_str(&txt).expect(&format!("無法解析檔案: {}", file_name));
    for cur_flow in all_flows.tt_flows.iter() {
        tsns.push(Flow {
            id: 0.into(),
            size: cur_flow.size,
            src: cur_flow.src,
            dst: cur_flow.dst,
            period: cur_flow.period,
            max_delay: cur_flow.max_delay,
            spec_data: TSNData {
                offset: cur_flow.offset,
            },
        });
    }
    for cur_flow in all_flows.avb_flows.iter() {
        avbs.push(Flow {
            id: 0.into(),
            size: cur_flow.size,
            src: cur_flow.src,
            dst: cur_flow.dst,
            period: cur_flow.period,
            max_delay: cur_flow.max_delay,
            spec_data: AVBData {
                avb_class: if cur_flow.avb_type == 'A' {
                    AVBClass::A
                } else if cur_flow.avb_type == 'B' {
                    AVBClass::B
                } else {
                    panic!("AVB type 必需為 `A` 或 `B`");
                },
            },
        });
    }
}

pub fn read_topo_from_file(file_name: &str) -> Network {
    let txt = fs::read_to_string(file_name).expect(&format!("找不到檔案: {}", file_name));
    let json: GraphJSON =
        serde_json::from_str(&txt).expect(&format!("無法解析檔案: {}", file_name));
    let mut network = Network::new();
    network.add_nodes(json.host_cnt, json.switch_cnt);
    network.add_edges(json.edges);
    network
}

#[derive(Serialize, Deserialize)]
struct AllFlows {
    tt_flows: Vec<RawTSNFlow>,
    avb_flows: Vec<RawAVBFlow>,
}
#[derive(Serialize, Deserialize)]
struct RawTSNFlow {
    size: usize,
    src: usize,
    dst: usize,
    period: u32,
    max_delay: u32,
    offset: u32,
}
#[derive(Serialize, Deserialize)]
struct RawAVBFlow {
    size: usize,
    src: usize,
    dst: usize,
    period: u32,
    max_delay: u32,
    avb_type: char,
}

#[derive(Serialize, Deserialize)]
struct GraphJSON {
    host_cnt: usize,
    switch_cnt: usize,
    edges: Vec<(usize, usize, f64)>,
}

