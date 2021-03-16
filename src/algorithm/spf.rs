use std::rc::Rc;
use std::time::Instant;
use crate::network::Network;
use crate::component::NetworkWrapper;
use super::Algorithm;
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
    fn configure(&mut self, _wrapper: &mut NetworkWrapper, _deadline: Instant) {
    }
    fn build_wrapper(&self, network: Network) -> NetworkWrapper {
        let yens = Rc::clone(&self.yens);
        let closure = move |src, dst, _| {
            yens.kth_shortest_path(src, dst, 0).unwrap() as *const Vec<usize>
        };
        NetworkWrapper::new(network, closure)
    }
}
