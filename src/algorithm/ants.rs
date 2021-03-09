use super::RoutingAlgo;
use crate::utils::config::Config;
use crate::utils::stream::{AVBFlow, Flow, FlowEnum, FlowID, TSNFlow};
use crate::network::Network;
use crate::component::{NetworkWrapper, RoutingCost};
use super::aco::ACO;
use super::base::yens::YensAlgo;
use crate::MAX_K;
use crate::component::IFlowTable;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use super::aco_routing::do_aco;

fn get_src_dst(flow: &FlowEnum) -> (usize, usize) {
    match flow {
        FlowEnum::AVB(flow) => (flow.src, flow.dst),
        FlowEnum::TSN(flow) => (flow.src, flow.dst),
    }
}

pub struct AdamsAnt {
    pub aco: ACO,
    pub yens_algo: Rc<RefCell<YensAlgo>>,
    pub wrapper: NetworkWrapper,
    pub compute_time: u128,
}
impl AdamsAnt {
    pub fn new(g: Network) -> Self {
        let yens_algo = Rc::new(RefCell::new(YensAlgo::default()));
        let tmp_yens = yens_algo.clone();
        tmp_yens.borrow_mut().compute(&g, MAX_K);
        let wrapper = NetworkWrapper::new(g, move |flow_enum, k| {
            let (src, dst) = get_src_dst(flow_enum);
            tmp_yens.borrow().kth_shortest_path(src, dst, k).unwrap() as *const Vec<usize>
        });

        AdamsAnt {
            aco: ACO::new(0, MAX_K, None),
            yens_algo,
            compute_time: 0,
            wrapper,
        }
    }
    pub fn get_candidate_count<T: Clone>(&self, flow: &Flow<T>) -> usize {
        self.yens_algo.borrow().count_shortest_paths(flow.src, flow.dst)
    }
}

impl RoutingAlgo for AdamsAnt {
    fn add_flows(&mut self, tsns: Vec<TSNFlow>, avbs: Vec<AVBFlow>) {
        // for flow in tsns.iter() {
        //     self.yens_algo
        //         .borrow_mut()
        //         .compute_once(flow.src, flow.dst);
        // }
        // for flow in avbs.iter() {
        //     self.yens_algo
        //         .borrow_mut()
        //         .compute_once(flow.src, flow.dst);
        // }
        let init_time = Instant::now();
        self.wrapper.insert(tsns, avbs, 0);

        self.aco
            .extend_state_len(self.wrapper.get_flow_table().get_max_id().0 + 1);

        do_aco(
            self,
            Config::get().t_limit - init_time.elapsed().as_micros(),
        );
        self.compute_time = init_time.elapsed().as_micros();
    }
    fn get_rerouted_flows(&self) -> &Vec<FlowID> {
        unimplemented!();
    }
    fn get_route(&self, id: FlowID) -> &Vec<usize> {
        self.wrapper.get_route(id)
    }
    fn show_results(&self) {
        println!("TT Flows:");
        for flow in self.wrapper.get_flow_table().iter_tsn() {
            let route = self.get_route(flow.id);
            println!("flow id = {:?}, route = {:?}", flow.id, route);
        }
        println!("AVB Flows:");
        for flow in self.wrapper.get_flow_table().iter_avb() {
            let route = self.get_route(flow.id);
            let cost = self.wrapper.compute_single_avb_cost(flow);
            println!(
                "flow id = {:?}, route = {:?} avb wcd / max latency = {:?}, reroute = {}",
                flow.id, route, cost.avb_wcd, cost.reroute_overhead
            );
        }
        let all_cost = self.wrapper.compute_all_cost();
        println!("the cost structure = {:?}", all_cost);
        println!("{}", all_cost.compute());
    }
    fn get_last_compute_time(&self) -> u128 {
        self.compute_time
    }
    fn get_cost(&self) -> RoutingCost {
        self.wrapper.compute_all_cost()
    }
}
