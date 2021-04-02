use crate::{cnc::Toolbox, component::Decision};
use crate::component::FlowTable;
use crate::network::Network;
use std::time::Instant;
use super::base::yens::Yens;
use super::Algorithm;


pub struct SPF {
    yens: Yens,
}


impl Algorithm for SPF {
    fn prepare(&mut self, decision: &mut Decision, flowtable: &FlowTable) {
        let input_candidates = flowtable.inputs()
            .map(|id| flowtable.ends(id))
            .map(|ends| self.yens.k_shortest_paths(ends.0, ends.1));
        decision.candidates.extend(input_candidates);
    }
    fn configure(&mut self, _decision: &mut Decision, _flowtable: &FlowTable, _deadline: Instant, _toolbox: Toolbox) {
    }
}

impl SPF {
    pub fn new(network: &Network) -> Self {
        let yens = Yens::new(&network, 1);
        SPF { yens }
    }
}
