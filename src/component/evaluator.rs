use crate::component::FlowTable;
use crate::component::GateCtrlList;
use crate::network::Network;
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
    pub fn compute_avb_wcd(&self, decision: &Decision, flowtable: &FlowTable, network: &Network, id: usize) -> u32 {
        let kth = decision.kth_next(id).unwrap();
        evaluate_avb_latency_for_kth(decision, flowtable, network, id, kth)
    }
    pub fn compute_single_avb_cost(&self, decision: &Decision, latest: &Decision, flowtable: &FlowTable, network: &Network, avb: usize) -> [f64; 4] {
        let spec = flowtable.avb_spec(avb)
            .expect("Failed to obtain AVB spec from TSN stream");
        let avb_wcd = self.compute_avb_wcd(decision, flowtable, network, avb) as f64 / spec.max_delay as f64;
        let mut avb_fail_cnt = 0;
        let mut reroute_cnt = 0;
        if avb_wcd >= 1.0 {
            // 逾時了！
            avb_fail_cnt += 1;
        }
        if is_rerouted(
            decision.kth_next(avb),
            latest.kth(avb),
        ) {
            reroute_cnt += 1;
        }
        let mut objs = [0.0; 4];
        objs[0] = decision.tsn_fail as u8 as f64;
        objs[1] = avb_fail_cnt as f64;
        objs[2] = reroute_cnt as f64;
        objs[3] = avb_wcd;
        objs
    }
    pub fn compute_all_cost(&self, decision: &Decision, latest: &Decision, flowtable: &FlowTable, network: &Network) -> [f64; 4] {
        let mut all_avb_fail_cnt = 0;
        let mut all_avb_wcd = 0.0;
        let mut all_reroute_cnt = 0;
        for &id in flowtable.tsns() {
            let t = decision.kth_next(id);
            if is_rerouted(t, latest.kth(id)) {
                all_reroute_cnt += 1;
            }
        }
        for &avb in flowtable.avbs() {
            let spec = flowtable.avb_spec(avb)
                .expect("Failed to obtain AVB spec from TSN stream");
            let wcd = self.compute_avb_wcd(decision, flowtable, network, avb);
            all_avb_wcd += wcd as f64 / spec.max_delay as f64;
            if wcd > spec.max_delay {
                // 逾時了！
                all_avb_fail_cnt += 1;
            }
            let t = decision.kth_next(avb);
            if is_rerouted(t, latest.kth(avb)) {
                all_reroute_cnt += 1;
            }
        }
        let mut objs = [0.0; 4];
        objs[0] = decision.tsn_fail as u8 as f64;
        objs[1] = all_avb_fail_cnt as f64 / flowtable.avbs().len() as f64;
        objs[2] = all_reroute_cnt as f64 / flowtable.len() as f64;
        objs[3] = all_avb_wcd / flowtable.avbs().len() as f64;
        objs
    }
    pub fn evaluate_cost_objectives(&self, decision: &Decision, latest: &Decision, flowtable: &FlowTable, network: &Network) -> (f64, [f64; 4]) {
        let objs = self.compute_all_cost(decision, latest, flowtable, network);
        let cost = objs.iter()
            .zip(self.weights.iter())
            .map(|(x, y)| x * y)
            .sum();
        (cost, objs)
    }
}

pub fn evaluate_avb_latency_for_kth(decision: &Decision, flowtable: &FlowTable, network: &Network, id: usize, kth: usize) -> u32 {
    compute_avb_latency(decision, network, id, kth, flowtable)
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
pub fn compute_avb_latency(
    decision: &Decision,
    network: &Network,
    id: usize,
    kth: usize,
    flowtable: &FlowTable,
) -> u32 {
    let route = decision.kth_route(id, kth);
    let gcl = &decision.allocated_tsns;
    let overlap_flow_id = decision.get_overlap_flows(route);
    let mut end_to_end_lanency = 0.0;
    for (i, (ends, bandwidth)) in network.get_links_id_bandwidth(route).into_iter().enumerate() {
        let wcd = wcd_on_single_link(id, bandwidth, flowtable, &overlap_flow_id[i]);
        end_to_end_lanency += wcd + tt_interfere_avb_single_link(ends, wcd as f64, gcl) as f64;
    }
    end_to_end_lanency as u32
}
fn wcd_on_single_link(
    avb: usize,
    bandwidth: f64,
    flowtable: &FlowTable,
    overlap_flow_id: &Vec<usize>,
) -> f64 {
    let spec = flowtable.avb_spec(avb)
        .expect("Failed to obtain AVB spec from TSN stream");
    let mut wcd = 0.0;
    // MAX None AVB
    wcd += MAX_BE_SIZE / bandwidth;
    // AVB 資料流最多只能佔用這樣的頻寬
    let bandwidth = MAX_AVB_SETTING * bandwidth;
    // On link
    wcd += spec.size as f64 / bandwidth;
    // Ohter AVB
    for &other_avb in overlap_flow_id.iter() {
        if other_avb != avb {
            let other_spec = flowtable.avb_spec(other_avb)
                .expect("Failed to obtain AVB spec from TSN stream");
            // 自己是 B 類或別人是 A 類，就有機會要等……換句話說，只有自己是 A 而別人是 B 不用等
            let self_type = spec.avb_type;
            let other_type = other_spec.avb_type;
            if self_type == 'B' || other_type == 'A' {
                wcd += other_spec.size as f64 / bandwidth;
            }
        }
    }
    wcd
}
fn tt_interfere_avb_single_link(ends: (usize, usize), wcd: f64, gcl: &GateCtrlList) -> u32 {
    let mut i_max = 0;
    let all_gce = gcl.get_gate_events(ends);
    for mut j in 0..all_gce.len() {
        let (mut i_cur, mut rem) = (0, wcd as i32);
        while rem >= 0 {
            let gce_ptr = all_gce[j];
            i_cur += gce_ptr.1;
            j += 1;
            if j == all_gce.len() {
                // TODO 應該要循環？
                break;
            }
            let gce_ptr_next = all_gce[j];
            rem -= gce_ptr_next.0 as i32 - (gce_ptr.0 + gce_ptr.1) as i32;
        }
        i_max = std::cmp::max(i_max, i_cur);
    }
    return i_max;
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
