use std::rc::Rc;
use std::time::Instant;
use super::Algorithm;
use crate::network::Network;
use crate::component::NetworkWrapper;
use super::aco::ACO;
use super::base::yens::YensAlgo;
use crate::MAX_K;
use super::aco_routing::do_aco;

pub struct AdamsAnt {
    pub aco: ACO,
    pub yens: Rc<YensAlgo>,
}
impl AdamsAnt {
    pub fn new(network: &Network) -> Self {
        let yens = YensAlgo::new(&network, MAX_K);
        AdamsAnt {
            aco: ACO::new(0, MAX_K, None),
            yens: Rc::new(yens),
        }
    }
    pub fn get_candidate_count(&self, src: usize, dst: usize) -> usize {
        self.yens.count_shortest_paths(src, dst)
    }
}

impl Algorithm for AdamsAnt {
    fn prepare(&mut self, wrapper: &mut NetworkWrapper) {
        for id in wrapper.inputs.clone() {
            let (src, dst) = wrapper.arena.ends(id);
            let candidates = self.yens.k_shortest_paths(src, dst);
            wrapper.candidates.push(candidates);
        }
    }
    fn configure(&mut self, wrapper: &mut NetworkWrapper, deadline: Instant) {
        let arena = Rc::clone(&wrapper.arena);
        self.aco
            .extend_state_len(arena.len());

        do_aco(
            wrapper,
            self,
            deadline,
        );
    }
}
