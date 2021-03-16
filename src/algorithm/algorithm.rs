use std::time::Instant;
use enum_dispatch::enum_dispatch;
use crate::network::Network;
use super::ants::AdamsAnt;
use super::ro::RO;
use super::spf::SPF;
use crate::component::NetworkWrapper;


#[enum_dispatch]
pub enum AlgorithmEnum {
    AdamsAnt,
    RO,
    SPF,
}

#[enum_dispatch(AlgorithmEnum)]
pub trait Algorithm {
    fn configure(&mut self, wrapper: &mut NetworkWrapper, deadline: Instant);
    fn build_wrapper(&self, network: Network) -> NetworkWrapper;
}
