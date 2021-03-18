use std::{rc::Rc, time::{Duration, Instant}};

use crate::algorithm::{AlgorithmEnum, Algorithm, AdamsAnt, RO, SPF};
use crate::component::NetworkWrapper;
use crate::component::RoutingCost;
use crate::network::Network;
use crate::utils::config::Config;
use crate::utils::stream::{TSN, AVB};

pub struct CNC {
    algorithm: AlgorithmEnum,
    wrapper: NetworkWrapper,
}

impl CNC {
    pub fn new(name: &str, graph: Network) -> Self {
        let algorithm: AlgorithmEnum = match name {
            "aco" => AdamsAnt::new(&graph).into(),
            "ro"  => RO::new(&graph).into(),
            "spf" => SPF::new(&graph).into(),
            _     => panic!("Failed specify an unknown routing algorithm"),
        };
        let wrapper = NetworkWrapper::new(graph);
        Self { algorithm, wrapper }
    }
    pub fn add_streams(&mut self, tsns: Vec<TSN>, avbs: Vec<AVB>) {
        let wrapper = &mut self.wrapper;
        wrapper.insert(tsns, avbs);
    }
    pub fn configure(&mut self) -> u128 {
        let wrapper = &mut self.wrapper;
        let limit = Duration::from_micros(Config::get().t_limit as u64);

        let start = Instant::now();
        self.algorithm.prepare(wrapper);
        wrapper.update_avb();
        wrapper.update_tsn();
        self.algorithm.configure(wrapper, start + limit);
        let elapsed = start.elapsed().as_micros();

        self.show_results();
        let cost = self.wrapper.compute_all_cost();
        RoutingCost::show_brief(vec![cost]);

        elapsed
    }
    fn show_results(&self) {
        let arena = Rc::clone(&self.wrapper.arena);
        println!("TT Flows:");
        for &id in arena.tsns.iter() {
            let route = self.wrapper.get_route(id);
            println!("flow id = FlowID({:?}), route = {:?}", id, route);
        }
        println!("AVB Flows:");
        for &id in arena.avbs.iter() {
            let route = self.wrapper.get_route(id);
            let cost = self.wrapper.compute_single_avb_cost(id);
            println!(
                "flow id = FlowID({:?}), route = {:?} avb wcd / max latency = {:?}, reroute = {}",
                id, route, cost.avb_wcd, cost.reroute_overhead
            );
        }
        let all_cost = self.wrapper.compute_all_cost();
        println!("the cost structure = {:?}", all_cost,);
        println!("{}", all_cost.compute());
    }
}
