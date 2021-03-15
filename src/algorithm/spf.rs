use std::rc::Rc;
use std::time::Instant;
use crate::utils::stream::{TSN, AVB};
use crate::network::Network;
use crate::component::NetworkWrapper;
use super::RoutingAlgo;
use super::base::yens::YensAlgo;

pub struct SPF {
    yens: Rc<YensAlgo>,
    compute_time: u128,
}

impl SPF {
    pub fn new(network: &Network) -> Self {
        let yens = YensAlgo::new(&network, 1);
        SPF {
            yens: Rc::new(yens),
            compute_time: 0,
        }
    }
}

impl RoutingAlgo for SPF {
    fn get_last_compute_time(&self) -> u128 {
        self.compute_time
    }
    fn add_flows(&mut self, wrapper: &mut NetworkWrapper, tsns: Vec<TSN>, avbs: Vec<AVB>) {
        let init_time = Instant::now();
        for flow in tsns.into_iter() {
            wrapper.insert(vec![flow], vec![], 0);
        }
        for flow in avbs.into_iter() {
            wrapper.insert(vec![], vec![flow], 0);
        }
        self.compute_time = init_time.elapsed().as_micros();
    }
    fn build_wrapper(&self, network: Network) -> NetworkWrapper {
        let yens = Rc::clone(&self.yens);
        let closure = move |src, dst, _| {
            yens.kth_shortest_path(src, dst, 0).unwrap() as *const Vec<usize>
        };
        NetworkWrapper::new(network, closure)
    }
}
