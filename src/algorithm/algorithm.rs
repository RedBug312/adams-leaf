use std::time::Instant;
use enum_dispatch::enum_dispatch;
use super::aco::AdamsAnt;
use super::ro::RO;
use super::spf::SPF;
use crate::component::{NetworkWrapper, flowtable::FlowArena};


pub type Eval<'a> = Box<dyn Fn(&mut NetworkWrapper) -> (f64, bool) + 'a>;


#[enum_dispatch]
pub enum AlgorithmEnum {
    AdamsAnt,
    RO,
    SPF,
}

#[enum_dispatch(AlgorithmEnum)]
pub trait Algorithm {
    fn prepare(&mut self, wrapper: &mut NetworkWrapper, arena: &FlowArena);
    fn configure(&mut self, wrapper: &mut NetworkWrapper, arena: &FlowArena, deadline: Instant, evaluate: Eval);
}
