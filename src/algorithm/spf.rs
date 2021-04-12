use crate::{cnc::Toolbox, component::Solution, network::Path};
use crate::component::FlowTable;
use crate::network::Network;
use std::time::Instant;
use super::base::yens::Yens;
use super::Algorithm;


pub struct SPF {
    yens: Yens,
}


impl Algorithm for SPF {
    fn candidates(&self, src: usize, dst: usize) -> &Vec<Path> {
        self.yens.k_shortest_paths(src, dst)
    }
    fn prepare(&mut self, _solution: &mut Solution, _flowtable: &FlowTable) {}
    fn configure(&mut self, _solution: &mut Solution, _deadline: Instant, _toolbox: Toolbox) {
    }
}

impl SPF {
    pub fn new(network: &Network) -> Self {
        let yens = Yens::new(&network, 1);
        SPF { yens }
    }
}
