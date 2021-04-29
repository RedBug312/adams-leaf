use crate::cnc::Toolbox;
use crate::component::Solution;
use crate::network::Network;
use crate::network::Path;
use std::time::Instant;
use super::Algorithm;
use super::base::yens::Yens;


pub struct SPF {
    yens: Yens,
}


impl Algorithm for SPF {
    fn candidates(&self, src: usize, dst: usize) -> &Vec<Path> {
        self.yens.k_shortest_paths(src.into(), dst.into())
    }
    fn configure(&mut self, solution: &mut Solution, _deadline: Instant, toolbox: Toolbox) {
        toolbox.evaluate_cost(solution);
    }
}

impl SPF {
    pub fn new(network: &Network) -> Self {
        let mut yens = Yens::new(&network, 1);
        yens.compute(&network);
        SPF { yens }
    }
}
