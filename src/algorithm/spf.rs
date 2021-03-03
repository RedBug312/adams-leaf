use super::RoutingAlgo;
use crate::utils::stream::{AVBFlow, Flow, FlowID, TSNFlow};
use crate::network::StreamAwareGraph;
use crate::component::{NetworkWrapper, RoutingCost};
use super::base::dijkstra::Dijkstra;
use std::time::Instant;
use crate::component::IFlowTable;

pub struct SPF {
    wrapper: NetworkWrapper<Vec<usize>>,
    dijkstra_algo: Dijkstra,
    compute_time: u128,
}

impl SPF {
    pub fn new(g: StreamAwareGraph) -> Self {
        let wrapper = NetworkWrapper::new(g.clone(), |_, route| route as *const Vec<usize>);
        let mut spf = SPF {
            wrapper,
            compute_time: 0,
            dijkstra_algo: Dijkstra::default(),
        };
        spf.dijkstra_algo.compute(&g);
        spf
    }
}

impl RoutingAlgo for SPF {
    fn get_last_compute_time(&self) -> u128 {
        self.compute_time
    }
    fn add_flows(&mut self, tsns: Vec<TSNFlow>, avbs: Vec<AVBFlow>) {
        let init_time = Instant::now();
        for flow in tsns.into_iter() {
            let route = self.get_shortest_route(&flow);
            self.wrapper.insert(vec![flow], vec![], route);
        }
        for flow in avbs.into_iter() {
            let route = self.get_shortest_route(&flow);
            self.wrapper.insert(vec![], vec![flow], route);
        }
        self.compute_time = init_time.elapsed().as_micros();
    }
    fn get_rerouted_flows(&self) -> &Vec<FlowID> {
        unimplemented!();
    }
    fn get_route(&self, id: FlowID) -> &Vec<usize> {
        self.wrapper.get_flow_table().get_info(id).unwrap()
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

impl SPF {
    fn get_shortest_route<T: Clone>(&mut self, flow: &Flow<T>) -> Vec<usize> {
        self.dijkstra_algo.shortest_path(flow.src, flow.dst).unwrap()
    }
}
