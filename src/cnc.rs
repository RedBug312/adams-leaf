use std::{rc::Rc, time::{Duration, Instant}};

use crate::{algorithm::{AlgorithmEnum, Algorithm, AdamsAnt, RO, SPF}, component::Evaluator};
use crate::component::NetworkWrapper;
use crate::component::RoutingCost;
use crate::network::Network;
use crate::utils::config::Config;
use crate::utils::stream::{TSN, AVB};

pub struct CNC {
    algorithm: AlgorithmEnum,
    evaluator: Evaluator,
    wrapper: NetworkWrapper,
}

impl CNC {
    pub fn new(name: &str, graph: Network) -> Self {
        let config = Config::get();
        let mut weights = [config.w0, config.w1, config.w2, config.w3];
        if name == "ro" {
            weights[2] = 0f64;
        }
        let algorithm: AlgorithmEnum = match name {
            "aco" => AdamsAnt::new(&graph).into(),
            "ro"  => RO::new(&graph).into(),
            "spf" => SPF::new(&graph).into(),
            _     => panic!("Failed specify an unknown routing algorithm"),
        };
        let wrapper = NetworkWrapper::new(graph);
        let evaluator = Evaluator::new(weights);
        Self { algorithm, wrapper, evaluator }
    }
    pub fn add_streams(&mut self, tsns: Vec<TSN>, avbs: Vec<AVB>) {
        let wrapper = &mut self.wrapper;
        wrapper.insert(tsns, avbs);
    }
    pub fn configure(&mut self) -> u128 {
        let wrapper = &mut self.wrapper;
        let evaluator = &self.evaluator;
        let limit = Duration::from_micros(Config::get().t_limit as u64);

        let evaluate = |w: &mut NetworkWrapper| {
            w.adopt_decision();  // where it's mutated
            let objs = evaluator.compute_all_cost(w).objectives();
            let early_exit = objs[1] == 0f64 && Config::get().fast_stop;
            let cost: f64 = objs.iter()
                .zip(evaluator.weights.iter())
                .map(|(x, y)| x * y)
                .sum();
            (cost, early_exit)
        };
        let evaluate = Box::new(evaluate);

        let start = Instant::now();
        self.algorithm.prepare(wrapper);
        wrapper.adopt_decision();
        self.algorithm.configure(wrapper, start + limit, evaluate);
        let elapsed = start.elapsed().as_micros();

        let wrapper = &self.wrapper;
        self.show_results();
        let cost = self.evaluator.compute_all_cost(wrapper);
        RoutingCost::show_brief(vec![cost]);

        elapsed
    }
    fn show_results(&self) {
        let arena = Rc::clone(&self.wrapper.arena);
        let wrapper = &self.wrapper;
        println!("TT Flows:");
        for &id in arena.tsns.iter() {
            let route = self.wrapper.get_route(id);
            println!("flow id = FlowID({:?}), route = {:?}", id, route);
        }
        println!("AVB Flows:");
        for &id in arena.avbs.iter() {
            let route = self.wrapper.get_route(id);
            let cost = self.evaluator.compute_single_avb_cost(wrapper, id);
            println!(
                "flow id = FlowID({:?}), route = {:?} avb wcd / max latency = {:?}, reroute = {}",
                id, route, cost.avb_wcd, cost.reroute_overhead
            );
        }
        let all_cost = self.evaluator.compute_all_cost(wrapper);
        println!("the cost structure = {:?}", all_cost,);
        println!("{}", all_cost.compute());
    }
}
