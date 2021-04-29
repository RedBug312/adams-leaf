use std::time::Instant;

use enum_dispatch::enum_dispatch;

use super::aco::ACO;
use super::ro::RO;
use super::spf::SPF;
use crate::cnc::Toolbox;
use crate::component::Solution;
use crate::network::Path;

#[enum_dispatch]
pub enum AlgorithmEnum { ACO, RO, SPF }


#[enum_dispatch(AlgorithmEnum)]
pub trait Algorithm {
    fn candidates(&self, src: usize, dst: usize) -> &Vec<Path>;
    fn configure(&mut self, solution: &mut Solution, deadline: Instant, toolbox: Toolbox);
}
