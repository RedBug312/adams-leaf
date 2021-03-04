use crate::algorithm::{AlgorithmEnum, RoutingAlgo, AdamsAnt, RO, SPF};
use crate::component::RoutingCost;
use crate::network::StreamAwareGraph;
use crate::utils::stream::{AVBFlow, TSNFlow};

pub struct CNC {
    algorithm: AlgorithmEnum,
    iteration: u32,
}

impl CNC {
    pub fn new(name: &str, graph: StreamAwareGraph) -> Self {
        let algorithm = match name {
            "aco" => AdamsAnt::new(graph).into(),
            "ro"  => RO::new(graph).into(),
            "spf" => SPF::new(graph).into(),
            _     => panic!("Failed specify an unknown routing algorithm"),
        };
        Self { algorithm, iteration: 0 }
    }
    pub fn add_streams(&mut self, tsns: Vec<TSNFlow>, avbs: Vec<AVBFlow>) {
        self.algorithm.add_flows(tsns, avbs);
    }
    pub fn configure(&mut self) {
        self.iteration += 1;

        self.algorithm.show_results();
        let computing_time = self.algorithm.get_last_compute_time();
        println!("--- #{} computing time: {} μs ---",
                 self.iteration, computing_time);

        let cost = self.algorithm.get_cost();
        RoutingCost::show_brief(vec![cost]);
    }
}
