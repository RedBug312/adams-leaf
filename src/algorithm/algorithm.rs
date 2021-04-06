use crate::component::FlowTable;
use crate::component::Decision;
use crate::cnc::Toolbox;
use enum_dispatch::enum_dispatch;
use std::time::Instant;
use super::aco::ACO;
use super::ro::RO;
use super::spf::SPF;


#[enum_dispatch]
pub enum AlgorithmEnum { ACO, RO, SPF }


#[enum_dispatch(AlgorithmEnum)]
pub trait Algorithm {
    fn prepare(&mut self, decision: &mut Decision, flowtable: &FlowTable);
    fn configure(&mut self, decision: &mut Decision, flowtable: &FlowTable, deadline: Instant, toolbox: Toolbox);
}
