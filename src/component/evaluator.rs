use crate::component::FlowTable;
use crate::component::GateCtrlList;
use crate::network::Edge;
use crate::network::Network;
use std::cmp::max;
use std::rc::{Rc, Weak};
use super::Decision;


/// AVB 資料流最多可以佔用的資源百分比（模擬 Credit Base Shaper 的效果）
const MAX_AVB_SETTING: f64 = 0.75;
/// BE 資料流最多可以多大
const MAX_BE_SIZE: f64 = 1500.0;


#[derive(Default)]
pub struct Evaluator {
    flowtable: Weak<FlowTable>,
    network: Weak<Network>,
    weights: [f64; 4],
}


impl Evaluator {
    pub fn new(weights: [f64; 4]) -> Self {
        Evaluator { weights, ..Default::default() }
    }
    pub fn flowtable(&self) -> Rc<FlowTable> {
        self.flowtable.upgrade().unwrap()
    }
    pub fn flowtable_mut(&mut self) -> &mut Weak<FlowTable> {
        &mut self.flowtable
    }
    pub fn network(&self) -> Rc<Network> {
        self.network.upgrade().unwrap()
    }
    pub fn network_mut(&mut self) -> &mut Weak<Network> {
        &mut self.network
    }
    pub fn evaluate_avb_wcd(&self, avb: usize, decision: &Decision) -> u32 {
        let kth = decision.kth_next(avb).unwrap();
        self.evaluate_avb_wcd_for_kth(avb, kth, decision)
    }
    pub fn evaluate_avb_objectives(&self, avb: usize, decision: &Decision, latest: &Decision) -> [f64; 4] {
        let flowtable = self.flowtable();
        let latest = latest.kth(avb);
        let current = decision.kth_next(avb);
        let wcd = self.evaluate_avb_wcd(avb, decision);
        let max = flowtable.avb_spec(avb).unwrap().deadline;

        let mut objs = [0.0; 4];
        objs[0] = decision.tsn_fail as u8 as f64;
        objs[1] = (wcd > max) as usize as f64;
        objs[2] = is_rerouted(latest, current) as usize as f64;
        objs[3] = wcd as f64 / max as f64;
        objs
    }
    pub fn evaluate_objectives(&self, decision: &Decision, latest: &Decision)
        -> [f64; 4] {
        let flowtable = self.flowtable();
        let mut all_rerouted_count = 0;
        let mut avb_failed_count = 0;
        let mut avb_normed_wcd_sum = 0.0;

        for either in 0..flowtable.len() {
            let latest = latest.kth(either);
            let current = decision.kth_next(either);
            all_rerouted_count += is_rerouted(latest, current) as usize;
        }
        for &avb in flowtable.avbs() {
            let wcd = self.evaluate_avb_wcd(avb, decision);
            let max = flowtable.avb_spec(avb).unwrap().deadline;
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
    pub fn evaluate_cost_objectives(&self, decision: &Decision, latest: &Decision)
        -> (f64, [f64; 4]) {
        let objs = self.evaluate_objectives(decision, latest);
        let cost = objs.iter()
            .zip(self.weights.iter())
            .map(|(x, y)| x * y)
            .sum();
        (cost, objs)
    }

    /// 計算 AVB 資料流的端對端延遲（包含 TT、BE 及其它 AVB 所造成的延遲）
    /// * `g` - 全局網路拓撲，每條邊上記錄其承載哪些資料流
    /// * `flow` - 該 AVB 資料流的詳細資訊
    /// * `route` - 該 AVB 資料流的路徑
    /// * `flow_table` - 資料流表。需注意的是，這裡僅用了資料流本身的資料，而未使用其隨附資訊
    /// TODO: 改用 FlowTable?
    /// * `gcl` - 所有 TT 資料流的 Gate Control List
    pub fn evaluate_avb_wcd_for_kth(&self, avb: usize, kth: usize, decision: &Decision) -> u32 {
        let flowtable = self.flowtable();
        let network = self.network();
        let route = decision.candidate(avb, kth);
        let gcl = &decision.allocated_tsns;
        let mut end_to_end = 0.0;
        for ends in route.windows(2) {
            let edge = network.edge(ends);
            let traversed_avbs = decision.traversed_avbs.get(&edge.ends)
                .map_or_else(|| vec![], |set| set.iter().cloned().collect());
            let mut per_hop = 0.0;
            per_hop += transmit_avb_itself(edge, avb, &flowtable);
            per_hop += interfere_from_be(edge);
            per_hop += interfere_from_avb(edge, avb, traversed_avbs, &flowtable);
            per_hop += interfere_from_tsn(edge, per_hop, gcl);
            end_to_end += per_hop;
        }
        end_to_end as u32
    }
}

#[inline]
fn is_rerouted(latest: Option<usize>, current: Option<usize>) -> bool {
    latest.is_some() && current != latest
}

fn transmit_avb_itself(edge: &Edge, avb: usize, flowtable: &FlowTable) -> f64 {
    let bandwidth = MAX_AVB_SETTING * edge.bandwidth;
    let spec = flowtable.avb_spec(avb).unwrap();
    spec.size as f64 / bandwidth
}

fn interfere_from_be(edge: &Edge) -> f64 {
    MAX_BE_SIZE / edge.bandwidth
}

// FIXME incomplete implemnetation
// "IEEE Standard for Local and metropolitan area networks--Audio Video Bridging (AVB) Systems," in
// IEEE Std 802.1BA-2011, pp.1-45, 30 Sept. 2011, doi: 10.1109/IEEESTD.2011.6032690.

fn interfere_from_avb(edge: &Edge, avb: usize, others: Vec<usize>,
    flowtable: &FlowTable) -> f64 {
    let mut interfere = 0.0;
    let bandwidth = MAX_AVB_SETTING * edge.bandwidth;
    let spec = flowtable.avb_spec(avb).unwrap();
    for other in others {
        if avb == other { continue; }
        let other_spec = flowtable.avb_spec(other).unwrap();
        if spec.class == 'B' || other_spec.class == 'A' {
            interfere += other_spec.size as f64 / bandwidth;
        }
    }
    interfere
}

// FIXME incomplete implemnetation
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
mod tests {
    use crate::algorithm::Algorithm;
    use crate::cnc::CNC;
    use crate::network::Network;
    use crate::utils::yaml;
    use crate::utils::stream::AVB;
    use super::*;

    fn setup() -> CNC {
        let mut network = Network::new();
        network.add_nodes(3, 0);
        network.add_edges(vec![(0, 1, 100.0), (1, 2, 100.0)]);
        let tsns = vec![];
        let avbs = vec![
            AVB::new(0, 2, 075, 10000, 200, 'A'),
            AVB::new(0, 2, 150, 10000, 200, 'A'),
            AVB::new(0, 2, 075, 10000, 200, 'B'),
        ];
        let config = yaml::load_config("data/config/default.yaml");
        let mut cnc = CNC::new(network, config);
        cnc.add_streams(tsns, avbs);
        cnc.algorithm.prepare(&mut cnc.decision, &cnc.flowtable);
        cnc
    }

    #[test]
    fn it_determines_if_rerouted() {
        assert_eq!(is_rerouted(None, None), false);
        assert_eq!(is_rerouted(None, Some(1)), false);
        assert_eq!(is_rerouted(Some(1), Some(1)), false);
        assert_eq!(is_rerouted(Some(1), Some(2)), true);
    }

    #[test]
    fn it_evaluates_avb_interfere() {
        let cnc = setup();
        let edge = cnc.network.edge(&[0, 1]);
        let flowtable = &cnc.flowtable;
        let mut decision = cnc.decision.clone();
        cnc.scheduler.configure(&mut decision);
        assert_eq!(interfere_from_avb(&edge, 0, vec![0, 1, 2], flowtable), 2.0);
        assert_eq!(interfere_from_avb(&edge, 1, vec![0, 1, 2], flowtable), 1.0);
        assert_eq!(interfere_from_avb(&edge, 2, vec![0, 1, 2], flowtable), 3.0);
    }

    #[test]
    fn it_evaluates_tsn_interfere() {
        let cnc = setup();
        let edge = cnc.network.edge(&[0, 1]);
        let mut decision = cnc.decision.clone();
        cnc.scheduler.configure(&mut decision);
        // GCL: 3 - - - - 4 - 5 5 -
        let mut gcl = GateCtrlList::new(10);
        gcl.insert_gate_evt(edge.ends, 3, 0..1);
        gcl.insert_gate_evt(edge.ends, 4, 5..6);
        gcl.insert_gate_evt(edge.ends, 5, 7..9);
        println!("{:?}", gcl);
        assert_eq!(interfere_from_tsn(&edge, 1.0, &gcl), 3.0);  // should be 2.0
        assert_eq!(interfere_from_tsn(&edge, 2.0, &gcl), 3.0);  // should be 3.0
        assert_eq!(interfere_from_tsn(&edge, 3.0, &gcl), 3.0);  // should be 4.0
    }
}
