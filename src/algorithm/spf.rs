use super::RoutingAlgo;
use crate::{MAX_K, utils::stream::{AVBFlow, FlowEnum, FlowID, TSNFlow}};
use crate::network::StreamAwareGraph;
use crate::component::{NetworkWrapper, RoutingCost};
use super::base::yens::YensAlgo;
use std::{cell::RefCell, rc::Rc, time::Instant};
use crate::component::IFlowTable;

fn get_src_dst(flow: &FlowEnum) -> (usize, usize) {
    match flow {
        FlowEnum::AVB(flow) => (flow.src, flow.dst),
        FlowEnum::TSN(flow) => (flow.src, flow.dst),
    }
}

pub struct SPF {
    compute_time: u128,
    wrapper: NetworkWrapper<usize>,
}

impl SPF {
    pub fn new(g: StreamAwareGraph) -> Self {
        let yens_algo = Rc::new(RefCell::new(YensAlgo::default()));
        let tmp_yens = yens_algo.clone();
        tmp_yens.borrow_mut().compute(&g, MAX_K);
        let wrapper = NetworkWrapper::new(g, move |flow_enum, _| {
            let (src, dst) = get_src_dst(flow_enum);
            tmp_yens.borrow().kth_shortest_path(src, dst, 0).unwrap()
                as *const Vec<usize>
        });
        SPF {
            compute_time: 0,
            wrapper,
        }
    }
}

impl RoutingAlgo for SPF {
    fn get_last_compute_time(&self) -> u128 {
        self.compute_time
    }
    fn add_flows(&mut self, tsns: Vec<TSNFlow>, avbs: Vec<AVBFlow>) {
        let init_time = Instant::now();
        for flow in tsns.into_iter() {
            self.wrapper.insert(vec![flow], vec![], 0);
        }
        for flow in avbs.into_iter() {
            self.wrapper.insert(vec![], vec![flow], 0);
        }
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
        for (flow, _) in self.wrapper.get_flow_table().iter_tsn() {
            let route = self.get_route(flow.id);
            println!("flow id = {:?}, route = {:?}", flow.id, route);
        }
        println!("AVB Flows:");
        for (flow, _) in self.wrapper.get_flow_table().iter_avb() {
            let route = self.get_route(flow.id);
            let cost = self.wrapper.compute_single_avb_cost(flow);
            println!(
                "flow id = {:?}, route = {:?} avb wcd / max latency = {:?}, reroute = {}",
                flow.id, route, cost.avb_wcd, cost.reroute_overhead
            );
        }
        let all_cost = self.wrapper.compute_all_cost();
        println!("the cost structure = {:?}", all_cost,);
        println!("{}", all_cost.compute());
    }
    fn get_cost(&self) -> RoutingCost {
        self.wrapper.compute_all_cost()
    }
}
