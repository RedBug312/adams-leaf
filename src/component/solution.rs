use std::collections::{HashMap, HashSet};
use crate::network::Network;
use crate::component::GateCtrlList;


const KTH_DEFAULT: usize = 0;
type Route = Vec<usize>;


/// 這個結構預期會被複製很多次，因此其中的每個元件都應儘可能想辦法降低複製成本
#[derive(Clone)]
pub struct Solution {
    pub selections: Vec<Select>,
    pub candidates: Vec<Vec<Route>>,
    pub allocated_tsns: GateCtrlList,
    pub traversed_avbs: HashMap<(usize, usize), HashSet<usize>>,
    pub tsn_fail: bool,
}

#[derive(Clone)]
pub enum Select {
    Pending(usize),
    Stay(usize),
    Switch(usize, usize),
}


impl Solution {
    pub fn new(graph: &Network) -> Self {
        let traversed_avbs = graph.edges.keys()
            .map(|&ends| (ends, HashSet::new()))
            .collect();
        Solution {
            selections: vec![],
            candidates: vec![],
            allocated_tsns: GateCtrlList::new(1),
            traversed_avbs,
            tsn_fail: false,
        }
    }
    pub fn select(&mut self, nth: usize, kth: usize) {
         self.selections[nth].select(kth);
    }
    pub fn confirm(&mut self) {
        self.selections.iter_mut()
            .for_each(|selection| selection.confirm());
    }
    pub fn selection(&self, nth: usize) -> &Select {
        debug_assert!(nth < self.selections.len());
        &self.selections[nth]
    }
    pub fn candidate(&self, stream: usize, kth: usize) -> &Route {
        &self.candidates[stream][kth]
    }
    pub fn candidates(&self, stream: usize) -> &Vec<Route> {
        &self.candidates[stream]
    }
    pub fn route(&self, stream: usize) -> &Route {
        let kth = self.selection(stream).current().unwrap();
        self.candidate(stream, kth)
    }
    pub fn resize(&mut self, len: usize) {
        let default = Select::Pending(KTH_DEFAULT);
        self.selections.resize(len, default);
    }
}

impl Select {
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
    pub fn is_pending(&self) -> bool {
        matches!(self, Select::Pending(_))
    }
    pub fn is_switch(&self) -> bool {
        matches!(self, Select::Switch(curr, next) if curr != next)
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

        assert_eq!(solution.route(0), &vec![0, 2, 3, 1]);
        assert_eq!(solution.route(1), &vec![0, 3, 1]);
        assert_eq!(solution.route(2), &vec![0, 2, 3, 1]);
    }
}
