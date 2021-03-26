use std::rc::Rc;
use std::time::Instant;
use crate::network::Network;
use crate::component::NetworkWrapper;
use super::{Algorithm, algorithm::Eval};
use super::base::yens::YensAlgo;


pub struct SPF {
    yens: Rc<YensAlgo>,
}


impl SPF {
    pub fn new(network: &Network) -> Self {
        let yens = YensAlgo::new(&network, 1);
        SPF {
            yens: Rc::new(yens),
        }
    }
}

impl Algorithm for SPF {
    fn prepare(&mut self, wrapper: &mut NetworkWrapper) {
        // split borrowing
        let inputs = wrapper.inputs.clone();
        let arena = &wrapper.arena;
        let candidates = &mut wrapper.candidates;

        let input_candidates = inputs
            .map(|id| arena.ends(id))
            .map(|ends| self.yens.k_shortest_paths(ends.0, ends.1));
        candidates.extend(input_candidates);
    }
    fn configure(&mut self, _wrapper: &mut NetworkWrapper, _deadline: Instant, _evaluate: Eval) {
    }
}
