use crate::algorithm::{AlgorithmEnum, Algorithm, ACO, RO, SPF};
use crate::component::Decision;
use crate::component::Evaluator;
use crate::component::FlowTable;
use crate::network::Network;
use crate::scheduler::Scheduler;
use crate::utils::config::Config;
use crate::utils::stream::{TSN, AVB};
use std::{rc::Weak, time::{Duration, Instant}};
use std::rc::Rc;


pub struct CNC {
    algorithm: AlgorithmEnum,
    scheduler: Scheduler,
    evaluator: Evaluator,
    flowtable: Rc<FlowTable>,
    decision: Decision,
    network: Rc<Network>,
}


impl CNC {
    pub fn new(name: &str, graph: Network, seed: u64) -> Self {
        let config = Config::get();
        let mut weights = [config.w0, config.w1, config.w2, config.w3];
        if name == "ro" {
            weights[2] = 0.0;
        }
        let algorithm: AlgorithmEnum = match name {
            "aco" => ACO::new(&graph, seed).into(),
            "ro"  => RO::new(&graph, seed).into(),
            "spf" => SPF::new(&graph).into(),
            _     => panic!("Failed specify an unknown routing algorithm"),
        };
        let evaluator = Evaluator::new(weights);
        let flowtable = Rc::new(FlowTable::new());
        let decision = Decision::new(&graph);
        let network = Rc::new(graph);
        let mut scheduler = Scheduler::new();
        *scheduler.flowtable_mut() = Rc::downgrade(&flowtable);
        *scheduler.network_mut() = Rc::downgrade(&network);
        Self { algorithm, scheduler, evaluator, decision, flowtable, network }
    }
    pub fn add_streams(&mut self, tsns: Vec<TSN>, avbs: Vec<AVB>) {
        *self.scheduler.flowtable_mut() = Weak::new();
        let flowtable = Rc::get_mut(&mut self.flowtable);
        debug_assert!(flowtable.is_some());  // ensure everyone drops their ownerships
        flowtable.unwrap().append(tsns, avbs);
        self.decision.resize(self.flowtable.len());
        *self.scheduler.flowtable_mut() = Rc::downgrade(&self.flowtable);
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
            scheduler.configure(decision);  // where it's mutated
            let (cost, objs) = evaluator.evaluate_cost_objectives(decision, latest, flowtable, network);
            let early_exit = objs[1] == 0.0 && Config::get().fast_stop;
            (cost, early_exit)
        };
        let evaluate = Box::new(evaluate);

        let start = Instant::now();
        self.algorithm.prepare(&mut current, flowtable);
        self.scheduler.configure(&mut current);  // should not schedule before routing
        self.algorithm.configure(&mut current, flowtable, network, start + limit, evaluate);
        let elapsed = start.elapsed().as_micros();

        self.show_results(&current);
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
            println!("flow id = FlowID({}), route = {:?}", id, route);
        }
        println!("AVB Flows:");
        for &id in flowtable.avbs() {
            let route = current.route(id);
            let objs = self.evaluator.evaluate_avb_objectives(current, latest, flowtable, network, id);
            println!(
                "flow id = FlowID({}), route = {:?} avb wcd / max latency = {:?}, reroute = {}",
                id, route, objs[3], objs[2]
            );
        }
        let (cost, objs) = self.evaluator.evaluate_cost_objectives(current, latest, flowtable, network);
        println!("with cost {:.2} and each objective {:.2?}", cost, objs);
    }
}
