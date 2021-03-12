use super::{compute_avb_latency, NetworkWrapper, FlowTable};
use crate::utils::{config::Config, stream::FlowID};
use crate::utils::stream::AVBFlow;

#[derive(Clone, Copy, Debug)]
pub struct RoutingCost {
    pub tsn_schedule_fail: bool,
    pub avb_fail_cnt: u32,
    pub avb_wcd: f64,
    pub reroute_overhead: u32,
    pub avb_cnt: usize,
    pub tsn_cnt: usize,
}

impl RoutingCost {
    pub fn compute(&self) -> f64 {
        let config = Config::get();
        let cost = self.compute_without_reroute_cost();
        cost + config.w2 * self.reroute_overhead as f64 / (self.avb_cnt + self.tsn_cnt) as f64
    }
    pub fn compute_without_reroute_cost(&self) -> f64 {
        let config = Config::get();
        let mut cost = 0.0;
        if self.tsn_schedule_fail {
            cost += config.w0;
        }
        cost += config.w1 * self.avb_fail_cnt as f64 / self.avb_cnt as f64;
        cost += config.w3 * self.avb_wcd / self.avb_cnt as f64;
        cost
    }
    pub fn show_brief(list: Vec<Self>) {
        let mut all_avb_fail_cnt = 0;
        let mut all_avb_wcd = 0.0;
        let mut all_reroute_cnt = 0;
        let mut all_cost = 0.0;
        let times = list.len() as f64;
        println!(
            "{0: <10} {1: <10} {2: <10} {3: <20} total cost",
            "", "#avb fail", "#reroute", "sum of wcd/deadline"
        );
        for (i, cost) in list.iter().enumerate() {
            if cost.tsn_schedule_fail {
                println!("#{}:\tTSN Schedule Fail!", i);
            } else {
                all_avb_fail_cnt += cost.avb_fail_cnt;
                all_reroute_cnt += cost.reroute_overhead;
                all_avb_wcd += cost.avb_wcd;
                all_cost += cost.compute();
                println!(
                    "{0: <10} {1: <10} {2: <10} {3: <20} {4}",
                    format!("test #{}", i),
                    cost.avb_fail_cnt,
                    cost.reroute_overhead,
                    cost.avb_wcd,
                    cost.compute()
                );
            }
        }
        println!(
            "{0: <10} {1: <10} {2: <10} {3: <20} {4}",
            "average:",
            all_avb_fail_cnt as f64 / times,
            all_reroute_cnt as f64 / times,
            all_avb_wcd / times,
            all_cost / times,
        );
    }
}

pub trait Calculator {
    fn _compute_avb_wcd(&self, flow: &AVBFlow, route: Option<usize>) -> u32;
    fn _compute_single_avb_cost(&self, flow: &AVBFlow) -> RoutingCost;
    fn _compute_all_cost(&self) -> RoutingCost;
}

impl Calculator for NetworkWrapper {
    fn _compute_avb_wcd(&self, flow: &AVBFlow, route: Option<usize>) -> u32 {
        let (src, dst) = self.flow_table.ends(flow.id);
        let route_t = route.unwrap_or(self.flow_table.get_info(flow.id).unwrap());
        let route = unsafe {
            let r = (self.get_route_func)(src, dst, route_t);
            &*r
        };
        compute_avb_latency(&self.graph, flow, route, &self.flow_table, &self.gcl)
    }
    fn _compute_single_avb_cost(&self, flow: &AVBFlow) -> RoutingCost {
        let avb_wcd = self._compute_avb_wcd(flow, None) as f64 / flow.max_delay as f64;
        let mut avb_fail_cnt = 0;
        let mut reroute_cnt = 0;
        if avb_wcd >= 1.0 {
            // 逾時了！
            avb_fail_cnt += 1;
        }
        if is_rerouted(
            flow.id,
            self.flow_table.get_info(flow.id).unwrap(),
            self.old_new_table.as_ref().unwrap(),
        ) {
            reroute_cnt += 1;
        }
        RoutingCost {
            tsn_schedule_fail: self.tsn_fail,
            avb_cnt: 1,
            tsn_cnt: 0,
            avb_fail_cnt,
            avb_wcd,
            reroute_overhead: reroute_cnt,
        }
    }
    fn _compute_all_cost(&self) -> RoutingCost {
        let mut all_avb_fail_cnt = 0;
        let mut all_avb_wcd = 0.0;
        let mut all_reroute_cnt = 0;
        for flow in self.flow_table.iter_tsn() {
            let t = self.flow_table.get_info(flow.id)
                .expect("Failed get info from flowtable");
            if is_rerouted(flow.id, t, self.old_new_table.as_ref().unwrap()) {
                all_reroute_cnt += 1;
            }
        }
        for flow in self.flow_table.iter_avb() {
            let wcd = self._compute_avb_wcd(flow, None);
            all_avb_wcd += wcd as f64 / flow.max_delay as f64;
            if wcd > flow.max_delay {
                // 逾時了！
                all_avb_fail_cnt += 1;
            }
            let t = self.flow_table.get_info(flow.id)
                .expect("Failed get info from flowtable");
            if is_rerouted(flow.id, t, self.old_new_table.as_ref().unwrap()) {
                all_reroute_cnt += 1;
            }
        }
        RoutingCost {
            tsn_schedule_fail: self.tsn_fail,
            avb_cnt: self.flow_table.get_avb_cnt(),
            tsn_cnt: self.flow_table.get_tsn_cnt(),
            avb_fail_cnt: all_avb_fail_cnt,
            avb_wcd: all_avb_wcd,
            reroute_overhead: all_reroute_cnt,
        }
    }
}

fn is_rerouted(id: FlowID, route: usize, old_new_table: &FlowTable) -> bool {
    if let Some(old_route) = old_new_table.get_info(id) {
        route != old_route
    } else {
        false
    }
}
