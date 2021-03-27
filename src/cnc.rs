use crate::algorithm::{AlgorithmEnum, Algorithm, ACO, RO, SPF};
use crate::component::Decision;
use crate::component::Evaluator;
use crate::component::FlowTable;
use crate::component::RoutingCost;
use crate::network::Network;
use crate::scheduler::Scheduler;
use crate::utils::config::Config;
use crate::utils::stream::{TSN, AVB};
use std::time::{Duration, Instant};


pub struct CNC {
    algorithm: AlgorithmEnum,
    scheduler: Scheduler,
    evaluator: Evaluator,
    flowtable: FlowTable,
    decision: Decision,
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
            "aco" => ACO::new(&graph).into(),
            "ro"  => RO::new(&graph).into(),
            "spf" => SPF::new(&graph).into(),
            _     => panic!("Failed specify an unknown routing algorithm"),
        };
        let scheduler = Scheduler::new();
        let evaluator = Evaluator::new(weights);
        let flowtable = FlowTable::new();
        let decision = Decision::new(&graph);
        let network = graph;
        Self { algorithm, scheduler, evaluator, decision, flowtable, network }
    }
    pub fn add_streams(&mut self, tsns: Vec<TSN>, avbs: Vec<AVB>) {
        self.flowtable.append(tsns, avbs);
        self.decision.resize(self.flowtable.len());
    }
    pub fn configure(&mut self) -> u128 {
        let scheduler = &self.scheduler;
        let evaluator = &self.evaluator;
        let flowtable = &self.flowtable;
        let network = &self.network;
        let latest = &self.decision;

        let limit = Duration::from_micros(Config::get().t_limit as u64);
        let mut current = latest.clone();

        let evaluate = |decision: &mut Decision| {
            scheduler.configure(decision, flowtable, network);  // where it's mutated
            let objs = evaluator.compute_all_cost(decision, latest, flowtable, network).objectives();
            let early_exit = objs[1] == 0f64 && Config::get().fast_stop;
            let cost: f64 = objs.iter()
                .zip(evaluator.weights.iter())
                .map(|(x, y)| x * y)
                .sum();
            (cost, early_exit)
        };
        let evaluate = Box::new(evaluate);

        let start = Instant::now();
        self.algorithm.prepare(&mut current, flowtable);
        self.scheduler.configure(&mut current, flowtable, network);  // should not schedule before routing
        self.algorithm.configure(&mut current, flowtable, network, start + limit, evaluate);
        let elapsed = start.elapsed().as_micros();

        self.show_results(&current);
        let cost = self.evaluator.compute_all_cost(&current, latest, flowtable, network);
        RoutingCost::show_brief(vec![cost]);
        self.decision = current;

        elapsed
    }
    fn show_results(&self, current: &Decision) {
        let flowtable = &self.flowtable;
        let latest = &self.decision;
        let network = &self.network;
        println!("TT Flows:");
        for &id in flowtable.tsns() {
            let route = current.route(id);
            println!("flow id = FlowID({:?}), route = {:?}", id, route);
        }
        println!("AVB Flows:");
        for &id in flowtable.avbs() {
            let route = current.route(id);
            let cost = self.evaluator.compute_single_avb_cost(current, latest, flowtable, network, id);
            println!(
                "flow id = FlowID({:?}), route = {:?} avb wcd / max latency = {:?}, reroute = {}",
                id, route, cost.avb_wcd, cost.reroute_overhead
            );
        }
        let all_cost = self.evaluator.compute_all_cost(current, latest, flowtable, network);
        println!("the cost structure = {:?}", all_cost,);
        println!("{}", all_cost.compute());
    }
}
