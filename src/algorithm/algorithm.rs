use enum_dispatch::enum_dispatch;
use super::ants::AdamsAnt;
use super::ro::RO;
use super::spf::SPF;
use crate::utils::stream::{AVBFlow, TSNFlow};
use crate::component::RoutingCost;

#[enum_dispatch]
pub enum AlgorithmEnum {
    AdamsAnt,
    RO,
    SPF,
}

#[enum_dispatch(AlgorithmEnum)]
pub trait RoutingAlgo {
    fn add_flows(&mut self, tsns: Vec<TSNFlow>, avbs: Vec<AVBFlow>);
    fn get_rerouted_flows(&self) -> &Vec<usize>;
    fn get_route(&self, id: usize) -> &Vec<usize>;
    fn show_results(&self);
    fn get_last_compute_time(&self) -> u128;
    fn get_cost(&self) -> RoutingCost;
}
