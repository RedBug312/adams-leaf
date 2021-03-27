use std::time::Instant;
use enum_dispatch::enum_dispatch;
use super::aco::ACO;
use super::ro::RO;
use super::spf::SPF;
use crate::component::FlowTable;
use crate::component::Decision;
use crate::network::Network;


#[enum_dispatch]
pub enum AlgorithmEnum { ACO, RO, SPF }
pub type Eval<'a> = Box<dyn Fn(&mut Decision) -> (f64, bool) + 'a>;


#[enum_dispatch(AlgorithmEnum)]
pub trait Algorithm {
    fn prepare(&mut self, decision: &mut Decision, flowtable: &FlowTable);
    fn configure(&mut self, decision: &mut Decision, flowtable: &FlowTable, network: &Network, deadline: Instant, evaluate: Eval);
}
