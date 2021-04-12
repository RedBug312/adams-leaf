use crate::algorithm::{ACO, Algorithm, AlgorithmEnum, RO, SPF};
use crate::component::{Evaluator, FlowTable, Solution};
use crate::network::Network;
use crate::scheduler::Scheduler;
use crate::utils::config::Config;
use crate::utils::stream::{TSN, AVB};
use std::fmt::Write;
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};


pub struct CNC {
    pub algorithm: AlgorithmEnum,
    pub scheduler: Scheduler,
    pub evaluator: Evaluator,
    pub flowtable: Rc<FlowTable>,
    pub solution: Solution,
    #[allow(dead_code)]
    pub network: Rc<Network>,
    pub config: Config,
}

pub struct Toolbox<'a> {
    scheduler: &'a Scheduler,
    evaluator: &'a Evaluator,
    latest: &'a Solution,
    config: &'a Config,
}


impl CNC {
    pub fn new(graph: Network, config: Config) -> Self {
        let mut weights = config.weights;
        if config.algorithm == "ro" {
            weights[2] = 0.0;
        }
        let algorithm: AlgorithmEnum = match config.algorithm.as_str() {
            "aco" => ACO::new(&graph, config.seed, config.parameters.clone()).into(),
            "ro"  => RO::new(&graph, config.seed).into(),
            "spf" => SPF::new(&graph).into(),
            _     => panic!("Failed specify an unknown routing algorithm"),
        };
        let flowtable = Rc::new(FlowTable::new());
        let mut solution = Solution::new(&graph);
        let network = Rc::new(graph);
        solution.flowtable = Rc::downgrade(&flowtable);
        solution.network = Rc::downgrade(&network);
        let scheduler = Scheduler::new();
        let evaluator = Evaluator::new(weights);
        Self { algorithm, scheduler, evaluator, solution, flowtable, network, config }
    }
    pub fn add_streams(&mut self, tsns: Vec<TSN>, avbs: Vec<AVB>) {
        self.solution.flowtable = Weak::new();
        // ensure everyone drops their ownerships
        debug_assert!(Rc::weak_count(&self.flowtable) == 0);
        let flowtable = Rc::get_mut(&mut self.flowtable).unwrap();
        flowtable.append(tsns, avbs);
        flowtable.append_candidates(&self.algorithm);
        self.solution.resize(self.flowtable.len());
        self.solution.flowtable = Rc::downgrade(&self.flowtable);
    }
    pub fn configure(&mut self) -> u128 {
        let scheduler = &self.scheduler;
        let evaluator = &self.evaluator;
        let flowtable = &self.flowtable;
        let latest = &self.solution;
        let config = &self.config;

        let toolbox = Toolbox::pack(scheduler, evaluator, latest, config);
        let timeout = Duration::from_micros(config.timeout);
        let mut current = latest.clone();

        let start = Instant::now();
        self.algorithm.prepare(&mut current, flowtable);
        self.algorithm.configure(&mut current, start + timeout, toolbox);
        let elapsed = start.elapsed().as_micros();

        self.show_results(&current);
        self.solution = current;

        elapsed
    }
    fn show_results(&self, current: &Solution) {
        let flowtable = &self.flowtable;
        let latest = &self.solution;
        let mut msg = String::new();

        let (cost, objs) = self.evaluator.evaluate_cost_objectives(current, latest);

        writeln!(msg, "TSN streams").unwrap();
        for &tsn in flowtable.tsns() {
            let outcome = if current.outcome(tsn).is_unschedulable()
                { "failed" } else { "ok" };
            let kth = current.selection(tsn).current().unwrap();
            let route = flowtable.candidate(tsn, kth);
            writeln!(msg, "- stream #{:02} {}, with route #{} {:?}",
                     tsn, outcome, kth, route).unwrap();
        }
        writeln!(msg, "AVB streams").unwrap();
        for &avb in flowtable.avbs() {
            let objs = self.evaluator.evaluate_avb_objectives(avb, current, latest);
            let max = flowtable.avb_spec(avb).unwrap().deadline as f64;
            let outcome = if objs[1] == 0.0 { "ok" } else { "failed" };
            let reroute = if objs[2] == 0.0 { "" } else { "*" };
            let kth = current.selection(avb).current().unwrap();
            let route = flowtable.candidate(avb, kth);
            writeln!(msg, "- stream #{:02} {} ({:02.0}%), with route #{}{} {:?}",
                     avb, outcome, objs[3] / max * 100.0, kth, reroute, route).unwrap();
        }
        writeln!(msg, "the solution has cost {:.2} and each objective {:.2?}",
                 cost, objs).unwrap();
        print!("{}", msg);
    }
}

impl<'a> Toolbox<'a> {
    pub fn pack(scheduler: &'a Scheduler, evaluator: &'a Evaluator,
                latest: &'a Solution, config: &'a Config) -> Self {
        Toolbox { scheduler, evaluator, latest, config }
    }
    pub fn evaluate_wcd(&'a self, avb: usize, kth: usize, solution: &Solution) -> u32 {
        self.evaluator.evaluate_avb_wcd_for_kth(avb, kth, solution)
    }
    pub fn evaluate_cost(&'a self, solution: &mut Solution) -> (f64, bool) {
        self.scheduler.configure(solution);  // where it's mutated
        let (cost, objs) = self.evaluator.evaluate_cost_objectives(solution, self.latest);
        let stop = self.config.early_stop && objs[0] == 0.0 && objs[1] == 0.0;
        (cost, stop)
    }
}
