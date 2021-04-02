use std::collections::{HashMap, HashSet};
use crate::network::Network;
use crate::component::GateCtrlList;


const KTH_DEFAULT: usize = 0;
type Route = Vec<usize>;


/// 這個結構預期會被複製很多次，因此其中的每個元件都應儘可能想辦法降低複製成本
#[derive(Clone)]
pub struct Decision {
    choices: Vec<Choice>,
    pub candidates: Vec<Vec<Route>>,
    pub allocated_tsns: GateCtrlList,
    pub traversed_avbs: HashMap<(usize, usize), HashSet<usize>>,
    pub tsn_fail: bool,
}

#[derive(Clone)]
enum Choice {
    Pending(usize),
    Stay(usize),
    Switch(usize, usize),
}


impl Decision {
    pub fn new(graph: &Network) -> Self {
        let traversed_avbs = graph.edges.keys()
            .map(|&ends| (ends, HashSet::new()))
            .collect();
        Decision {
            choices: vec![],
            candidates: vec![],
            allocated_tsns: GateCtrlList::new(1),
            traversed_avbs,
            tsn_fail: false,
        }
    }
    pub fn kth(&self, stream: usize) -> Option<usize> {
        self.choices[stream].kth()
    }
    pub fn kth_next(&self, stream: usize) -> Option<usize> {
        self.choices[stream].kth_next()
    }
    pub fn kth_route(&self, stream: usize, kth: usize) -> &Route {
        &self.candidates[stream][kth]
    }
    pub fn route(&self, stream: usize) -> &Route {
        let kth = self.kth(stream).unwrap();
        self.kth_route(stream, kth)
    }
    pub fn route_next(&self, stream: usize) -> &Route {
        let kth_next = self.kth_next(stream).unwrap();
        self.kth_route(stream, kth_next)
    }
    pub fn resize(&mut self, len: usize) {
        let default = Choice::Pending(KTH_DEFAULT);
        self.choices.resize(len, default);
    }
    pub fn pick(&mut self, stream: usize, kth: usize) {
        self.choices[stream].pick(kth);
    }
    pub fn confirm(&mut self) {
        self.choices.iter_mut()
            .for_each(|choice| choice.confirm());
    }
    pub fn filter_pending<'a>(&'a self, source: &'a Vec<usize>)
        -> impl Iterator<Item=usize> + 'a {
        source.iter().cloned()
            .filter(move |&id| matches!(self.choices[id],
                    Choice::Pending(_)))
    }
    pub fn filter_switch<'a>(&'a self, source: &'a Vec<usize>)
        -> impl Iterator<Item=usize> + 'a {
        source.iter().cloned()
            .filter(move |&id| matches!(self.choices[id],
                    Choice::Switch(prev, next) if prev != next))
    }
}

impl Choice {
    fn kth(&self) -> Option<usize> {
        match self {
            Choice::Pending(_)      => None,
            Choice::Stay(prev)      => Some(*prev),
            Choice::Switch(prev, _) => Some(*prev),
        }
    }
    fn kth_next(&self) -> Option<usize> {
        match self {
            Choice::Pending(next)   => Some(*next),
            Choice::Stay(prev)      => Some(*prev),
            Choice::Switch(_, next) => Some(*next),
        }
    }
    fn pick(&mut self, next: usize) {
        *self = match self {
            Choice::Pending(_)      => Choice::Pending(next),
            Choice::Stay(prev)      => Choice::Switch(*prev, next),
            Choice::Switch(prev, _) => Choice::Switch(*prev, next),
        };
    }
    fn confirm(&mut self) {
        *self = match self {
            Choice::Pending(next)   => Choice::Stay(*next),
            Choice::Stay(prev)      => Choice::Stay(*prev),
            Choice::Switch(_, next) => Choice::Stay(*next),
        };
    }
}


#[cfg(test)]
mod tests {
    use crate::algorithm::Algorithm;
    use crate::cnc::CNC;
    use crate::utils::json;
    use crate::utils::stream::TSN;

    fn init() -> CNC {
        let network = json::load_network("test_graph.json");
        let tsns = vec![
            TSN::new(0, 4, 100, 100, 100, 0),
            TSN::new(0, 4, 100, 150, 150, 0),
            TSN::new(1, 2, 100, 200, 200, 0),
        ];
        let avbs = vec![];
        let mut cnc = CNC::new("aco", network, 0);
        cnc.add_streams(tsns, avbs);
        cnc.algorithm.prepare(&mut cnc.decision, &cnc.flowtable);
        cnc
    }

    #[test]
    fn test_pick_kth() {
        let mut cnc = init();
        let decision = &mut cnc.decision;
        decision.pick(1, 1);

        assert_eq!(decision.kth(0), None);
        assert_eq!(decision.kth(1), None);
        assert_eq!(decision.kth(2), None);

        assert_eq!(decision.kth_next(0), Some(0));
        assert_eq!(decision.kth_next(1), Some(1));
        assert_eq!(decision.kth_next(2), Some(0));
    }

    #[test]
    fn test_confirm_decision() {
        let mut cnc = init();
        let decision = &mut cnc.decision;
        decision.pick(1, 1);
        decision.confirm();

        assert_eq!(decision.kth(0), Some(0));
        assert_eq!(decision.kth(1), Some(1));
        assert_eq!(decision.kth(2), Some(0));

        assert_eq!(decision.route(0), &vec![0, 4]);
        assert_eq!(decision.route(1), &vec![0, 5, 4]);
        assert_eq!(decision.route(2), &vec![1, 3, 2]);
    }
}
