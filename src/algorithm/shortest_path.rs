use std::time::Instant;

use super::base::yens::Yens;
use super::Algorithm;
use crate::cnc::Toolbox;
use crate::component::Solution;
use crate::network::{Network, Path};

pub struct SPF {
    yens: Yens,
}

impl Algorithm for SPF {
    fn candidates(&self, src: usize, dst: usize) -> &Vec<Path> {
        self.yens.k_shortest_paths(src.into(), dst.into())
    }
    fn configure(&mut self, mut last_run: Solution, _deadline: Instant, toolbox: Toolbox) -> Solution {
        toolbox.evaluate_cost(&mut last_run);
        last_run
    }
}

impl SPF {
    pub fn new(network: &Network) -> Self {
        let mut yens = Yens::new(&network, 1);
        yens.compute(&network);
        SPF { yens }
    }
}
