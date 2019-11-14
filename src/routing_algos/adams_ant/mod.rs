use std::time::Instant;

use super::time_and_tide::schedule_online;
use super::{Flow, FlowTable, RoutingAlgo, GCL};
use crate::graph_util::{Graph, StreamAwareGraph};
use crate::util::aco::ACO;
use crate::util::YensAlgo;
use crate::T_LIMIT;

mod cost_calculate;
pub(self) use cost_calculate::{compute_all_avb_cost, compute_avb_cost, AVBCostResult};

mod aco_routing;
use aco_routing::do_aco;

type FT = FlowTable<usize>;
const K: usize = 20;

pub struct AdamsAnt<'a> {
    aco: ACO,
    g: StreamAwareGraph,
    flow_table: FT,
    yens_algo: YensAlgo<'a, usize, StreamAwareGraph>,
    gcl: GCL,
    avb_count: usize,
    tt_count: usize,
    compute_time: u128,
}
impl<'a> AdamsAnt<'a> {
    pub fn new(g: &'a StreamAwareGraph, flow_table: Option<FT>, gcl: Option<GCL>) -> Self {
        let flow_table = flow_table.unwrap_or(FlowTable::new());
        let gcl = gcl.unwrap_or(GCL::new(1, g.get_edge_cnt()));
        AdamsAnt {
            gcl,
            flow_table,
            aco: ACO::new(0, K, None),
            g: g.clone(),
            yens_algo: YensAlgo::new(g, K),
            avb_count: 0,
            tt_count: 0,
            compute_time: 0,
        }
    }
    pub fn get_kth_route(&self, flow_id: usize, k: usize) -> &Vec<usize> {
        let flow = self.flow_table.get_flow(flow_id);
        self.yens_algo.get_kth_route(*flow.src(), *flow.dst(), k)
    }
    fn get_candidate_count(&self, flow: &Flow) -> usize {
        self.yens_algo.get_route_count(*flow.src(), *flow.dst())
    }
    fn schedule_online(
        &self,
        gcl: &mut GCL,
        og_table: &mut FT,
        changed_table: &FT,
    ) -> Result<bool, ()> {
        let _self = self as *const Self;
        unsafe {
            schedule_online(og_table, changed_table, gcl, |flow, &k| {
                let r = (*_self).get_kth_route(*flow.id(), k);
                (*_self).g.get_links_id_bandwidth(r)
            })
        }
    }
    unsafe fn update_flowid_on_route(&self, remember: bool, flow_id: usize, k: usize) {
        let _g = &self.g as *const StreamAwareGraph as *mut StreamAwareGraph;
        let route = self.get_kth_route(flow_id, k);
        (*_g).update_flowid_on_route(remember, flow_id, route);
    }
}

impl<'a> RoutingAlgo for AdamsAnt<'a> {
    fn add_flows(&mut self, flows: Vec<Flow>) {
        let init_time = Instant::now();
        self.add_flows_in_time(flows, T_LIMIT);
        self.compute_time = init_time.elapsed().as_micros();
    }
    fn del_flows(&mut self, flows: Vec<Flow>) {
        unimplemented!();
    }
    fn get_rerouted_flows(&self) -> &Vec<usize> {
        unimplemented!();
    }
    fn get_route(&self, id: usize) -> &Vec<usize> {
        let k = *self.flow_table.get_info(id);
        self.get_kth_route(id, k)
    }
    fn show_results(&self) {
        println!("TT Flows:");
        self.flow_table.foreach(false, |flow, &route_k| {
            let route = self.get_kth_route(*flow.id(), route_k);
            println!("flow id = {}, route = {:?}", *flow.id(), route);
        });
        println!("AVB Flows:");
        self.flow_table.foreach(true, |flow, &route_k| {
            let route = self.get_kth_route(*flow.id(), route_k);
            let cost = self.compute_avb_cost(flow, Some(route_k)).1;
            println!(
                "flow id = {}, route = {:?} cost = {}",
                *flow.id(),
                route,
                cost
            );
        });
        println!("total avb cost = {}", self.compute_all_avb_cost().1);
    }
    fn get_last_compute_time(&self) -> u128 {
        self.compute_time
    }
}

impl<'a> AdamsAnt<'a> {
    pub fn compute_avb_cost(&self, flow: &Flow, k: Option<usize>) -> AVBCostResult {
        compute_avb_cost(self, flow, k, &self.flow_table, &self.gcl)
    }
    pub fn compute_all_avb_cost(&self) -> AVBCostResult {
        compute_all_avb_cost(self, &self.flow_table, &self.gcl)
    }
    pub fn add_flows_in_time(&mut self, flows: Vec<Flow>, t_limit: u128) {
        let mut max_id = 0;
        self.flow_table.insert(flows.clone(), 0);
        let mut reconf = self.flow_table.clone_into_changed_table();
        for flow in flows.iter() {
            max_id = std::cmp::max(max_id, *flow.id());
            self.yens_algo.compute_routes(*flow.src(), *flow.dst());
            reconf.update_info(*flow.id(), 0);
            if flow.is_avb() {
                self.avb_count += 1;
            } else {
                self.tt_count += 1;
            }
        }
        self.aco.extend_state_len(max_id + 1);

        do_aco(self, t_limit, reconf);
        self.g.forget_all_flows();
        self.flow_table.foreach(true, |flow, r| unsafe {
            self.update_flowid_on_route(true, *flow.id(), *r)
        });
    }
}
