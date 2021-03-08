use crate::algorithm::{AlgorithmEnum, RoutingAlgo, AdamsAnt, RO, SPF};
use crate::component::RoutingCost;
use crate::network::Network;
use crate::utils::stream::{AVBFlow, TSNFlow};

pub struct CNC {
    algorithm: AlgorithmEnum,
}

impl CNC {
    pub fn new(name: &str, graph: Network) -> Self {
        let algorithm = match name {
            "aco" => AdamsAnt::new(graph).into(),
            "ro"  => RO::new(graph).into(),
            "spf" => SPF::new(graph).into(),
            _     => panic!("Failed specify an unknown routing algorithm"),
        };
        Self { algorithm }
    }
    pub fn add_streams(&mut self, tsns: Vec<TSNFlow>, avbs: Vec<AVBFlow>) {
        self.algorithm.add_flows(tsns, avbs);
    }
    pub fn configure(&mut self) -> u128 {
        self.algorithm.show_results();
        let cost = self.algorithm.get_cost();
        RoutingCost::show_brief(vec![cost]);

        self.algorithm.get_last_compute_time()
    }
}
