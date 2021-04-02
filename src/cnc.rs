use crate::algorithm::{ACO, Algorithm, AlgorithmEnum, RO, SPF};
use crate::component::{Decision, Evaluator, FlowTable};
use crate::network::Network;
use crate::scheduler::Scheduler;
use crate::utils::config::Config;
use crate::utils::stream::{TSN, AVB};
use std::time::{Duration, Instant};
use std::rc::{Rc, Weak};


pub struct CNC {
    pub algorithm: AlgorithmEnum,
    pub scheduler: Scheduler,
    pub evaluator: Evaluator,
    pub flowtable: Rc<FlowTable>,
    pub decision: Decision,
    #[allow(dead_code)]
    pub network: Rc<Network>,
}

pub struct Toolbox<'a> {
    scheduler: &'a Scheduler,
    evaluator: &'a Evaluator,
    latest: &'a Decision,
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
        let flowtable = Rc::new(FlowTable::new());
        let decision = Decision::new(&graph);
        let network = Rc::new(graph);
        let mut scheduler = Scheduler::new();
        *scheduler.flowtable_mut() = Rc::downgrade(&flowtable);
        *scheduler.network_mut() = Rc::downgrade(&network);
        let mut evaluator = Evaluator::new(weights);
        *evaluator.flowtable_mut() = Rc::downgrade(&flowtable);
        *evaluator.network_mut() = Rc::downgrade(&network);
        Self { algorithm, scheduler, evaluator, decision, flowtable, network }
    }
    pub fn add_streams(&mut self, tsns: Vec<TSN>, avbs: Vec<AVB>) {
        *self.scheduler.flowtable_mut() = Weak::new();
        *self.evaluator.flowtable_mut() = Weak::new();
        // ensure everyone drops their ownerships
        debug_assert!(Rc::weak_count(&self.flowtable) == 0);
        let flowtable = Rc::get_mut(&mut self.flowtable).unwrap();
        flowtable.append(tsns, avbs);
        self.decision.resize(self.flowtable.len());
        *self.scheduler.flowtable_mut() = Rc::downgrade(&self.flowtable);
        *self.evaluator.flowtable_mut() = Rc::downgrade(&self.flowtable);
    }
    pub fn configure(&mut self) -> u128 {
        let scheduler = &self.scheduler;
        let evaluator = &self.evaluator;
        let flowtable = &self.flowtable;
        let latest = &self.decision;

        let timeout = Duration::from_micros(Config::get().t_limit as u64);
        let mut current = latest.clone();

        let start = Instant::now();
        self.algorithm.prepare(&mut current, flowtable);
        self.scheduler.configure(&mut current);  // should not schedule before routing
        let toolbox = Toolbox::pack(scheduler, evaluator, latest);
        self.algorithm.configure(&mut current, flowtable, start + timeout, toolbox);
        let elapsed = start.elapsed().as_micros();

        self.show_results(&current);
        self.decision = current;

        elapsed
    }
    fn show_results(&self, current: &Decision) {
        let flowtable = &self.flowtable;
        let latest = &self.decision;
        println!("TT Flows:");
        for &tsn in flowtable.tsns() {
            let route = current.route(tsn);
            println!("flow id = FlowID({}), route = {:?}", tsn, route);
        }
        println!("AVB Flows:");
        for &avb in flowtable.avbs() {
            let route = current.route(avb);
            let objs = self.evaluator.evaluate_avb_objectives(avb, current, latest);
            println!(
                "flow id = FlowID({}), route = {:?} avb wcd / max latency = {:?}, reroute = {}",
                avb, route, objs[3], objs[2]
            );
        }
        let (cost, objs) = self.evaluator.evaluate_cost_objectives(current, latest);
        println!("with cost {:.2} and each objective {:.2?}", cost, objs);
    }
}

impl<'a> Toolbox<'a> {
    pub fn pack(scheduler: &'a Scheduler, evaluator: &'a Evaluator, latest: &'a Decision) -> Self {
        Toolbox { scheduler, evaluator, latest }
    }
    pub fn evaluate_wcd(&'a self, avb: usize, kth: usize, decision: &Decision) -> u32 {
        self.evaluator.evaluate_avb_wcd_for_kth(avb, kth, decision)
    }
    pub fn evaluate_cost(&'a self, decision: &mut Decision) -> (f64, bool) {
        self.scheduler.configure(decision);  // where it's mutated
        let (cost, objs) = self.evaluator.evaluate_cost_objectives(decision, self.latest);
        let early_exit = objs[1] == 0.0 && Config::get().fast_stop;
        (cost, early_exit)
    }
}
