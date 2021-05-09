mod adams_ants;
mod base;
mod routing_optimism;
mod shortest_path;

pub use adams_ants::ACO;
pub use routing_optimism::RO;
pub use shortest_path::SPF;

use std::time::Instant;
use enum_dispatch::enum_dispatch;
use crate::cnc::Toolbox;
use crate::component::Solution;
use crate::network::Path;

#[enum_dispatch]
pub enum AlgorithmEnum { ACO, RO, SPF }

#[enum_dispatch(AlgorithmEnum)]
pub trait Algorithm {
    fn candidates(&self, src: usize, dst: usize) -> &Vec<Path>;
    fn configure(&mut self, last_run: Solution, deadline: Instant, toolbox: Toolbox) -> Solution;
}
