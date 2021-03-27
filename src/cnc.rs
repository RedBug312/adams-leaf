use std::time::{Duration, Instant};
use crate::algorithm::{AlgorithmEnum, Algorithm, AdamsAnt, RO, SPF};
use crate::component::NetworkWrapper;
use crate::component::RoutingCost;
use crate::component::{Evaluator, flowtable::FlowArena};
use crate::network::Network;
use crate::scheduler::Scheduler;
use crate::utils::config::Config;
use crate::utils::stream::{TSN, AVB};

pub struct CNC {
    algorithm: AlgorithmEnum,
    scheduler: Scheduler,
    evaluator: Evaluator,
    wrapper: NetworkWrapper,
    arena: FlowArena,
    network: Network,
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
        let scheduler = Scheduler::new();
        let evaluator = Evaluator::new(weights);
        let wrapper = NetworkWrapper::new(&graph);
        let arena = FlowArena::new();
        let network = graph;
        Self { algorithm, scheduler, evaluator, wrapper, arena, network }
    }
    pub fn add_streams(&mut self, tsns: Vec<TSN>, avbs: Vec<AVB>) {
        self.arena.append(tsns, avbs);
        self.wrapper.resize(self.arena.len());
    }
    pub fn configure(&mut self) -> u128 {
        let scheduler = &self.scheduler;
        let evaluator = &self.evaluator;
        let arena = &self.arena;
        let network = &self.network;
        let latest = &self.wrapper;

        let limit = Duration::from_micros(Config::get().t_limit as u64);
        let mut current = latest.clone();

        let evaluate = |decision: &mut NetworkWrapper| {
            scheduler.configure(decision, arena, network);  // where it's mutated
            let objs = evaluator.compute_all_cost(decision, latest, arena, network).objectives();
            let early_exit = objs[1] == 0f64 && Config::get().fast_stop;
            let cost: f64 = objs.iter()
                .zip(evaluator.weights.iter())
                .map(|(x, y)| x * y)
                .sum();
            (cost, early_exit)
        };
        let evaluate = Box::new(evaluate);

        let start = Instant::now();
        self.algorithm.prepare(&mut current, arena);
        self.scheduler.configure(&mut current, arena, network);  // should not schedule before routing
        self.algorithm.configure(&mut current, arena, network, start + limit, evaluate);
        let elapsed = start.elapsed().as_micros();

        self.show_results(&current);
        let cost = self.evaluator.compute_all_cost(&current, latest, arena, network);
        RoutingCost::show_brief(vec![cost]);
        self.wrapper = current;

        elapsed
    }
    fn show_results(&self, current: &NetworkWrapper) {
        let arena = &self.arena;
        let latest = &self.wrapper;
        let network = &self.network;
        println!("TT Flows:");
        for &id in arena.tsns() {
            let route = current.route(id);
            println!("flow id = FlowID({:?}), route = {:?}", id, route);
        }
        println!("AVB Flows:");
        for &id in arena.avbs() {
            let route = current.route(id);
            let cost = self.evaluator.compute_single_avb_cost(current, latest, arena, network, id);
            println!(
                "flow id = FlowID({:?}), route = {:?} avb wcd / max latency = {:?}, reroute = {}",
                id, route, cost.avb_wcd, cost.reroute_overhead
            );
        }
        let all_cost = self.evaluator.compute_all_cost(current, latest, arena, network);
        println!("the cost structure = {:?}", all_cost,);
        println!("{}", all_cost.compute());
    }
}
