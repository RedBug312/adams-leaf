use crate::algorithm::{ACO, Algorithm, AlgorithmEnum, RO, SPF};
use crate::component::{Decision, Evaluator, FlowTable};
use crate::network::Network;
use crate::scheduler::Scheduler;
use crate::utils::config::Config;
use crate::utils::stream::{TSN, AVB};
use std::fmt::{Error, Write};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};


pub struct CNC {
    pub algorithm: AlgorithmEnum,
    pub scheduler: Scheduler,
    pub evaluator: Evaluator,
    pub flowtable: Rc<FlowTable>,
    pub decision: Decision,
    #[allow(dead_code)]
    pub network: Rc<Network>,
    pub config: Config,
    pub summary: String,
}

pub struct Toolbox<'a> {
    scheduler: &'a Scheduler,
    evaluator: &'a Evaluator,
    latest: &'a Decision,
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
        let decision = Decision::new(&graph);
        let network = Rc::new(graph);
        let summary = String::new();
        let mut scheduler = Scheduler::new();
        *scheduler.flowtable_mut() = Rc::downgrade(&flowtable);
        *scheduler.network_mut() = Rc::downgrade(&network);
        let mut evaluator = Evaluator::new(weights);
        *evaluator.flowtable_mut() = Rc::downgrade(&flowtable);
        *evaluator.network_mut() = Rc::downgrade(&network);
        Self { algorithm, scheduler, evaluator, decision, flowtable, network, config, summary }
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
        let config = &self.config;

        let toolbox = Toolbox::pack(scheduler, evaluator, latest, config);
        let timeout = Duration::from_micros(config.timeout);
        let mut current = latest.clone();

        let start = Instant::now();
        self.algorithm.prepare(&mut current, flowtable);
        self.algorithm.configure(&mut current, flowtable, start + timeout, toolbox);
        let elapsed = start.elapsed().as_micros();

        self.summary = self.summarize(&current).unwrap();
        self.decision = current;

        elapsed
    }
    fn summarize(&self, current: &Decision) -> Result<String, Error> {
        let flowtable = &self.flowtable;
        let latest = &self.decision;
        let mut msg = String::new();

        let (cost, objs) = self.evaluator.evaluate_cost_objectives(current, latest);

        writeln!(msg, "TSN streams")?;
        for &tsn in flowtable.tsns() {
            let outcome = if objs[0] == 0.0 { "ok" } else { "failed" };
            let kth = current.kth(tsn).unwrap();
            let route = current.route(tsn);
            writeln!(msg, "- stream #{:02} {}, with route #{} {:?}",
                     tsn, outcome, kth, route)?;
        }
        writeln!(msg, "AVB streams")?;
        for &avb in flowtable.avbs() {
            let objs = self.evaluator.evaluate_avb_objectives(avb, current, latest);
            let outcome = if objs[3] <= 1.0 { "ok" } else { "failed" };
            let reroute = if objs[2] == 0.0 { "" } else { "*" };
            let kth = current.kth(avb).unwrap();
            let route = current.route(avb);
            writeln!(msg, "- stream #{:02} {} ({:02.0}%), with route #{}{} {:?}",
                     avb, outcome, objs[3] * 100.0, kth, reroute, route)?;
        }
        writeln!(msg, "the solution has cost {:.2} and each objective {:.2?}",
                 cost, objs)?;
        Ok(msg)
    }
}

impl<'a> Toolbox<'a> {
    pub fn pack(scheduler: &'a Scheduler, evaluator: &'a Evaluator,
                latest: &'a Decision, config: &'a Config) -> Self {
        Toolbox { scheduler, evaluator, latest, config }
    }
    pub fn evaluate_wcd(&'a self, avb: usize, kth: usize, decision: &Decision) -> u32 {
        self.evaluator.evaluate_avb_wcd_for_kth(avb, kth, decision)
    }
    pub fn evaluate_cost(&'a self, decision: &mut Decision) -> (f64, bool) {
        self.scheduler.configure(decision);  // where it'msg mutated
        let (cost, objs) = self.evaluator.evaluate_cost_objectives(decision, self.latest);
        let stop = self.config.early_stop && objs[1] == 0.0;
        (cost, stop)
    }
}
