use std::rc::Rc;
use std::time::Instant;
use super::RoutingAlgo;
use crate::utils::config::Config;
use crate::utils::stream::{TSN, AVB};
use crate::network::Network;
use crate::component::NetworkWrapper;
use super::aco::ACO;
use super::base::yens::YensAlgo;
use crate::MAX_K;
use super::aco_routing::do_aco;

pub struct AdamsAnt {
    pub aco: ACO,
    pub yens: Rc<YensAlgo>,
    pub compute_time: u128,
}
impl AdamsAnt {
    pub fn new(network: &Network) -> Self {
        let yens = YensAlgo::new(&network, MAX_K);
        AdamsAnt {
            aco: ACO::new(0, MAX_K, None),
            yens: Rc::new(yens),
            compute_time: 0,
        }
    }
    pub fn get_candidate_count(&self, src: usize, dst: usize) -> usize {
        self.yens.count_shortest_paths(src, dst)
    }
}

impl RoutingAlgo for AdamsAnt {
    fn add_flows(&mut self, wrapper: &mut NetworkWrapper, tsns: Vec<TSN>, avbs: Vec<AVB>) {
        // for flow in tsns.iter() {
        //     self.yens
        //         .borrow_mut()
        //         .compute_once(flow.src, flow.dst);
        // }
        // for flow in avbs.iter() {
        //     self.yens
        //         .borrow_mut()
        //         .compute_once(flow.src, flow.dst);
        // }
        let init_time = Instant::now();
        wrapper.insert(tsns, avbs, 0);

        let arena = Rc::clone(&wrapper.arena);
        self.aco
            .extend_state_len(arena.len());

        do_aco(
            wrapper,
            self,
            Config::get().t_limit - init_time.elapsed().as_micros(),
        );
        self.compute_time = init_time.elapsed().as_micros();
    }
    fn get_last_compute_time(&self) -> u128 {
        self.compute_time
    }
    fn build_wrapper(&self, network: Network) -> NetworkWrapper {
        let yens = Rc::clone(&self.yens);
        let closure = move |src, dst, k| {
            yens.kth_shortest_path(src, dst, k).unwrap() as *const Vec<usize>
        };
        NetworkWrapper::new(network, closure)
    }
}
