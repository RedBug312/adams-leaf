use std::{collections::{HashMap, HashSet}, rc::{Rc, Weak}};
use crate::network::Network;
use crate::component::GateCtrlList;

use super::FlowTable;


const KTH_DEFAULT: usize = 0;


/// 這個結構預期會被複製很多次，因此其中的每個元件都應儘可能想辦法降低複製成本
#[derive(Clone)]
pub struct Solution {
    selections: Vec<Select>,
    outcomes: Vec<Outcome>,
    pub allocated_tsns: GateCtrlList,
    pub traversed_avbs: HashMap<(usize, usize), HashSet<usize>>,
    pub flowtable: Weak<FlowTable>,
    pub network: Weak<Network>,
}

#[derive(Clone)]
pub enum Select {
    Pending(usize),
    Stay(usize),
    Switch(usize, usize),
}

#[derive(Clone)]
pub enum Outcome {
    Pending,
    Schedulable(usize),
    Unschedulable(usize),
}

impl Solution {
    pub fn new(graph: &Network) -> Self {
        let traversed_avbs = graph.edges.keys()
            .map(|&ends| (ends, HashSet::new()))
            .collect();
        Solution {
            selections: vec![],
            outcomes: vec![],
            allocated_tsns: GateCtrlList::new(1),
            traversed_avbs,
            flowtable: Weak::new(),
            network: Weak::new(),
        }
    }
    pub fn flowtable(&self) -> Rc<FlowTable> {
        self.flowtable.upgrade().unwrap()
    }
    pub fn network(&self) -> Rc<Network> {
        self.network.upgrade().unwrap()
    }
    pub fn select(&mut self, nth: usize, kth: usize) {
         self.selections[nth].select(kth);
    }
    pub fn selection(&self, nth: usize) -> &Select {
        debug_assert!(nth < self.selections.len());
        &self.selections[nth]
    }
    pub fn confirm(&mut self) {
        self.selections.iter_mut()
            .for_each(|selection| selection.confirm());
    }
    pub fn outcome(&self, nth: usize) -> &Outcome {
        debug_assert!(nth < self.outcomes.len());
        &self.outcomes[nth]
    }
    pub fn flag_schedulable(&mut self, nth: usize, kth: usize) {
        debug_assert!(nth < self.outcomes.len());
        self.outcomes[nth] = Outcome::Schedulable(kth);
    }
    pub fn flag_unschedulable(&mut self, nth: usize, kth: usize) {
        debug_assert!(nth < self.outcomes.len());
        self.outcomes[nth] = Outcome::Unschedulable(kth);
    }
    pub fn resize(&mut self, len: usize) {
        self.selections.resize(len, Select::Pending(KTH_DEFAULT));
        self.outcomes.resize(len, Outcome::Pending)
    }
}

impl Select {
    pub fn is_pending(&self) -> bool {
        matches!(self, Select::Pending(_))
    }
    pub fn is_switch(&self) -> bool {
        matches!(self, Select::Switch(curr, next) if curr != next)
    }
    pub fn current(&self) -> Option<usize> {
        match self {
            Select::Pending(_)      => None,
            Select::Stay(curr)      => Some(*curr),
            Select::Switch(curr, _) => Some(*curr),
        }
    }
    pub fn next(&self) -> Option<usize> {
        match self {
            Select::Pending(next)   => Some(*next),
            Select::Stay(curr)      => Some(*curr),
            Select::Switch(_, next) => Some(*next),
        }
    }
    fn select(&mut self, next: usize) {
        *self = match self {
            Select::Pending(_)      => Select::Pending(next),
            Select::Stay(curr)      => Select::Switch(*curr, next),
            Select::Switch(curr, _) => Select::Switch(*curr, next),
        };
    }
    fn confirm(&mut self) {
        *self = match self {
            Select::Pending(next)   => Select::Stay(*next),
            Select::Stay(curr)      => Select::Stay(*curr),
            Select::Switch(_, next) => Select::Stay(*next),
        };
    }
}

impl Outcome {
    pub fn is_schedulable(&self) -> bool {
        matches!(self, Outcome::Schedulable(_))
    }
    pub fn is_unschedulable(&self) -> bool {
        matches!(self, Outcome::Unschedulable(_) | Outcome::Pending)
    }
}

#[cfg(test)]
mod tests {
    use crate::algorithm::Algorithm;
    use crate::cnc::CNC;
    use crate::utils::yaml;
    use crate::utils::stream::TSN;

    fn setup() -> CNC {
        let network = yaml::load_network("data/network/trap.yaml");
        let tsns = vec![
            TSN::new(0, 1, 100, 100, 100, 0),
            TSN::new(0, 1, 100, 150, 150, 0),
            TSN::new(0, 1, 100, 200, 200, 0),
        ];
        let avbs = vec![];
        let config = yaml::load_config("data/config/default.yaml");
        let mut cnc = CNC::new(network, config);
        cnc.add_streams(tsns, avbs);
        cnc.algorithm.prepare(&mut cnc.solution, &cnc.flowtable);
        cnc
    }

    #[test]
    fn it_selects_kth() {
        let mut cnc = setup();
        let solution = &mut cnc.solution;
        solution.select(1, 1);

        assert_eq!(solution.selection(0).current(), None);
        assert_eq!(solution.selection(1).current(), None);
        assert_eq!(solution.selection(2).current(), None);

        assert_eq!(solution.selection(0).next(), Some(0));
        assert_eq!(solution.selection(1).next(), Some(1));
        assert_eq!(solution.selection(2).next(), Some(0));
    }

    #[test]
    fn it_confirms_solution() {
        let mut cnc = setup();
        let solution = &mut cnc.solution;
        solution.select(1, 1);
        solution.confirm();

        assert_eq!(solution.selection(0).current(), Some(0));
        assert_eq!(solution.selection(1).current(), Some(1));
        assert_eq!(solution.selection(2).current(), Some(0));

        assert_eq!(solution.selection(0).next(), Some(0));
        assert_eq!(solution.selection(1).next(), Some(1));
        assert_eq!(solution.selection(2).next(), Some(0));
    }
}
