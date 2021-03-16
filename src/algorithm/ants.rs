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
    fn build_wrapper(&self, network: Network) -> NetworkWrapper {
        let yens = Rc::clone(&self.yens);
        let closure = move |src, dst, k| {
            yens.kth_shortest_path(src, dst, k).unwrap() as *const Vec<usize>
        };
        NetworkWrapper::new(network, closure)
    }
}
