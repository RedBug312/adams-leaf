use crate::component::FlowTable;
use crate::component::GateCtrlList;
use crate::network::Edge;
use crate::network::Network;
use std::cmp::max;
use super::Decision;


/// AVB 資料流最多可以佔用的資源百分比（模擬 Credit Base Shaper 的效果）
const MAX_AVB_SETTING: f64 = 0.75;
/// BE 資料流最多可以多大
const MAX_BE_SIZE: f64 = 1500.0;


#[derive(Default)]
pub struct Evaluator {
    pub weights: [f64; 4],
}


impl Evaluator {
    pub fn new(weights: [f64; 4]) -> Self {
        Evaluator { weights }
    }
    pub fn evaluate_avb_wcd(&self, decision: &Decision, flowtable: &FlowTable, network: &Network, id: usize) -> u32 {
        let kth = decision.kth_next(id).unwrap();
        evaluate_avb_wcd_for_kth(id, kth, decision, flowtable, network)
    }
    pub fn evaluate_avb_objectives(&self, decision: &Decision, latest: &Decision, flowtable: &FlowTable, network: &Network, avb: usize) -> [f64; 4] {
        let latest = latest.kth(avb);
        let current = decision.kth_next(avb);
        let wcd = self.evaluate_avb_wcd(decision, flowtable, network, avb);
        let max = flowtable.avb_spec(avb).unwrap().max_delay;

        let mut objs = [0.0; 4];
        objs[0] = decision.tsn_fail as u8 as f64;
        objs[1] = (wcd > max) as usize as f64;
        objs[2] = is_rerouted(current, latest) as usize as f64;
        objs[3] = wcd as f64 / max as f64;
        objs
    }
    pub fn evaluate_objectives(&self, decision: &Decision, latest: &Decision, flowtable: &FlowTable, network: &Network) -> [f64; 4] {
        let mut all_rerouted_count = 0;
        let mut avb_failed_count = 0;
        let mut avb_normed_wcd_sum = 0.0;

        for either in 0..flowtable.len() {
            let latest = latest.kth(either);
            let current = decision.kth_next(either);
            all_rerouted_count += is_rerouted(current, latest) as usize;
        }
        for &avb in flowtable.avbs() {
            let wcd = self.evaluate_avb_wcd(decision, flowtable, network, avb);
            let max = flowtable.avb_spec(avb).unwrap().max_delay;
            avb_failed_count += (wcd > max) as usize;
            avb_normed_wcd_sum += wcd as f64 / max as f64;
        }

        let mut objs = [0.0; 4];
        objs[0] = decision.tsn_fail as u8 as f64;
        objs[1] = avb_failed_count as f64 / flowtable.avbs().len() as f64;
        objs[2] = all_rerouted_count as f64 / flowtable.len() as f64;
        objs[3] = avb_normed_wcd_sum / flowtable.avbs().len() as f64;
        objs
    }
    pub fn evaluate_cost_objectives(&self, decision: &Decision, latest: &Decision, flowtable: &FlowTable, network: &Network) -> (f64, [f64; 4]) {
        let objs = self.evaluate_objectives(decision, latest, flowtable, network);
        let cost = objs.iter()
            .zip(self.weights.iter())
            .map(|(x, y)| x * y)
            .sum();
        (cost, objs)
    }
}

fn is_rerouted(current: Option<usize>, latest: Option<usize>) -> bool {
    latest.is_some() && current != latest
}

/// 計算 AVB 資料流的端對端延遲（包含 TT、BE 及其它 AVB 所造成的延遲）
/// * `g` - 全局網路拓撲，每條邊上記錄其承載哪些資料流
/// * `flow` - 該 AVB 資料流的詳細資訊
/// * `route` - 該 AVB 資料流的路徑
/// * `flow_table` - 資料流表。需注意的是，這裡僅用了資料流本身的資料，而未使用其隨附資訊
/// TODO: 改用 FlowTable?
/// * `gcl` - 所有 TT 資料流的 Gate Control List
pub fn evaluate_avb_wcd_for_kth(
    avb: usize,
    kth: usize,
    decision: &Decision,
    flowtable: &FlowTable,
    network: &Network,
) -> u32 {
    let route = decision.kth_route(avb, kth);
    let gcl = &decision.allocated_tsns;
    let mut end_to_end = 0.0;
    for ends in route.windows(2) {
        let edge = network.edge(ends);
        let traversed_avbs = decision.traversed_avbs.get(&edge.ends)
            .map_or_else(|| vec![], |set| set.iter().cloned().collect());
        let mut per_hop = 0.0;
        per_hop += transmit_avb_itself(edge, avb, flowtable);
        per_hop += interfere_from_be(edge);
        per_hop += interfere_from_avb(edge, avb, traversed_avbs, flowtable);
        per_hop += interfere_from_tsn(edge, per_hop, gcl);
        end_to_end += per_hop;
    }
    end_to_end as u32
}

fn transmit_avb_itself(edge: &Edge, avb: usize, flowtable: &FlowTable) -> f64 {
    let bandwidth = MAX_AVB_SETTING * edge.bandwidth;
    let spec = flowtable.avb_spec(avb).unwrap();
    spec.size as f64 / bandwidth
}

fn interfere_from_be(edge: &Edge) -> f64 {
    MAX_BE_SIZE / edge.bandwidth
}

fn interfere_from_avb(edge: &Edge, avb: usize, others: Vec<usize>,
    flowtable: &FlowTable) -> f64 {
    let mut interfere = 0.0;
    let bandwidth = MAX_AVB_SETTING * edge.bandwidth;
    let spec = flowtable.avb_spec(avb).unwrap();
    for other in others {
        if avb == other { continue; }
        let other_spec = flowtable.avb_spec(other).unwrap();
        if spec.avb_type == 'B' || other_spec.avb_type == 'A' {
            interfere += other_spec.size as f64 / bandwidth;
        }
    }
    interfere
}

// Sune Mølgaard Laursen, Paul Pop, and Wilfried Steiner. 2016. Routing optimization of AVB streams
// in TSN networks. SIGBED Rev. 13, 4 (September 2016), 43–48.
// DOI:https://doi.org/10.1145/3015037.3015044

fn interfere_from_tsn(edge: &Edge, wcd: f64, gcl: &GateCtrlList) -> f64 {
    let mut max_interfere = 0;
    let events = gcl.get_gate_events(edge.ends);
    for i in 0..events.len() {
        let mut interfere = 0;
        let mut remained = wcd as i32;
        let mut j = i;
        while remained >= 0 {
            let curr = &events[j];
            interfere += curr.end - curr.start;
            j += 1;
            if j == events.len() {
                // TODO 應該要循環？
                break;
            }
            let next = &events[j];
            remained -= next.start as i32 - curr.end as i32;
        }
        max_interfere = max(max_interfere, interfere);
    }
    max_interfere as f64
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::network::*;

    fn init_settings() -> (MemorizingGraph, Vec<AVBFlow>, FlowTable<usize>, GCL) {
        use crate::flow::data::{AVBClass, AVBData};
        let mut g = StreamAwareGraph::new();
        g.add_host(Some(3));
        g.add_edge((0, 1), 100.0).unwrap();
        g.add_edge((1, 2), 100.0).unwrap();
        let flows = vec![
            AVBFlow {
                id: 0.into(),
                src: 0,
                dst: 2,
                size: 75,
                period: 10000,
                max_delay: 200,
                spec_data: AVBData {
                    avb_class: AVBClass::A,
                },
            },
            AVBFlow {
                id: 0.into(),
                src: 0,
                dst: 2,
                size: 150,
                period: 10000,
                max_delay: 200,
                spec_data: AVBData {
                    avb_class: AVBClass::A,
                },
            },
            AVBFlow {
                id: 0.into(),
                src: 0,
                dst: 2,
                size: 75,
                period: 10000,
                max_delay: 200,
                spec_data: AVBData {
                    avb_class: AVBClass::B,
                },
            },
        ];
        let flow_table = FlowTable::new();
        let gcl = GCL::new(10, g.get_edge_cnt());
        (MemorizingGraph::new(g), flows, flow_table, gcl)
    }
    fn build_flowid_vec(v: Vec<usize>) -> Vec<usize> {
        v.into_iter().map(|i| i.into()).collect()
    }
    #[test]
    fn test_single_link_avb() {
        let (_, flows, mut route_table, _) = init_settings();

        route_table.insert(vec![], flows, 0);

        assert_eq!(
            wcd_on_single_link(
                route_table.get_avb(0.into()).unwrap(),
                100.0,
                &route_table,
                &build_flowid_vec(vec![0, 2])
            ),
            (MAX_BE_SIZE / 100.0 + 1.0)
        );
        assert_eq!(
            wcd_on_single_link(
                route_table.get_avb(0.into()).unwrap(),
                100.0,
                &route_table,
                &build_flowid_vec(vec![1, 0, 2])
            ),
            (MAX_BE_SIZE / 100.0 + 1.0 + 2.0)
        );
        assert_eq!(
            wcd_on_single_link(
                route_table.get_avb(1.into()).unwrap(),
                100.0,
                &route_table,
                &build_flowid_vec(vec![1, 0, 2])
            ),
            (MAX_BE_SIZE / 100.0 + 1.0 + 2.0)
        );

        assert_eq!(
            wcd_on_single_link(
                route_table.get_avb(2.into()).unwrap(),
                100.0,
                &route_table,
                &build_flowid_vec(vec![1, 0, 2])
            ),
            (MAX_BE_SIZE / 100.0 + 1.0 + 2.0 + 1.0)
        );
    }
    #[test]
    fn test_endtoend_avb_without_gcl() {
        let (mut g, flows, mut flow_table, gcl) = init_settings();
        flow_table.insert(vec![], vec![flows[0].clone()], 0);
        g.update_flowid_on_route(true, 0.into(), &vec![0, 1, 2]);
        assert_eq!(
            compute_avb_latency(&g, &flows[0], &vec![0, 1, 2], &flow_table, &gcl),
            ((MAX_BE_SIZE / 100.0 + 1.0) * 2.0) as u32
        );

        flow_table.insert(vec![], vec![flows[1].clone()], 0);
        g.update_flowid_on_route(true, 1.into(), &vec![0, 1, 2]);
        assert_eq!(
            compute_avb_latency(&g, &flows[0], &vec![0, 1, 2], &flow_table, &gcl),
            ((MAX_BE_SIZE / 100.0 + 1.0 + 2.0) * 2.0) as u32
        );
    }
    #[test]
    fn test_endtoend_avb_with_gcl() {
        // 其實已經接近整合測試了 @@
        let (mut g, flows, mut flow_table, mut gcl) = init_settings();

        flow_table.insert(vec![], vec![flows[0].clone()], 0);
        g.update_flowid_on_route(true, 0.into(), &vec![0, 1, 2]);
        flow_table.insert(vec![], vec![flows[1].clone()], 0);
        g.update_flowid_on_route(true, 1.into(), &vec![0, 1, 2]);

        gcl.insert_gate_evt(0, 99.into(), 0, 0, 10);
        assert_eq!(
            compute_avb_latency(
                &g,
                flow_table.get_avb(0.into()).unwrap(),
                &vec![0, 1, 2],
                &flow_table,
                &gcl
            ),
            ((MAX_BE_SIZE / 100.0 + 1.0 + 2.0) * 2.0 + 10.0) as u32
        );

        gcl.insert_gate_evt(0, 99.into(), 0, 15, 5);
        assert_eq!(
            compute_avb_latency(
                &g,
                flow_table.get_avb(0.into()).unwrap(),
                &vec![0, 1, 2],
                &flow_table,
                &gcl
            ),
            ((MAX_BE_SIZE / 100.0 + 1.0 + 2.0) * 2.0 + 15.0) as u32
        );

        gcl.insert_gate_evt(2, 99.into(), 0, 100, 100);
        // 雖然這個關閉事件跟前面兩個不可能同時發生，但為了計算快速，還是假裝全部都發生了
        assert_eq!(
            compute_avb_latency(
                &g,
                flow_table.get_avb(0.into()).unwrap(),
                &vec![0, 1, 2],
                &flow_table,
                &gcl
            ),
            ((MAX_BE_SIZE / 100.0 + 1.0 + 2.0) * 2.0 + 115.0) as u32
        );
        assert_eq!(
            compute_avb_latency(
                &g,
                flow_table.get_avb(1.into()).unwrap(),
                &vec![0, 1, 2],
                &flow_table,
                &gcl
            ),
            ((MAX_BE_SIZE / 100.0 + 2.0 + 1.0) * 2.0 + 115.0) as u32
        );

        gcl.insert_gate_evt(0, 99.into(), 0, 100, 100);
        // 這個事件與同個埠口上的前兩個事件不可能同時發生，選比較久的（即這個事件）
        assert_eq!(
            compute_avb_latency(
                &g,
                flow_table.get_avb(0.into()).unwrap(),
                &vec![0, 1, 2],
                &flow_table,
                &gcl
            ),
            ((MAX_BE_SIZE / 100.0 + 1.0 + 2.0) * 2.0 + 200.0) as u32
        );
        assert_eq!(
            compute_avb_latency(
                &g,
                flow_table.get_avb(1.into()).unwrap(),
                &vec![0, 1, 2],
                &flow_table,
                &gcl
            ),
            ((MAX_BE_SIZE / 100.0 + 2.0 + 1.0) * 2.0 + 200.0) as u32
        );
    }
}
