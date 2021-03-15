use enum_dispatch::enum_dispatch;
use crate::network::Network;
use super::ants::AdamsAnt;
use super::ro::RO;
use super::spf::SPF;
use crate::{component::NetworkWrapper, utils::stream::{TSN, AVB}};

#[enum_dispatch]
pub enum AlgorithmEnum {
    AdamsAnt,
    RO,
    SPF,
}

#[enum_dispatch(AlgorithmEnum)]
pub trait RoutingAlgo {
    fn add_flows(&mut self, wrapper: &mut NetworkWrapper, tsns: Vec<TSN>, avbs: Vec<AVB>);
    fn get_last_compute_time(&self) -> u128;
    fn build_wrapper(&self, network: Network) -> NetworkWrapper;
}
