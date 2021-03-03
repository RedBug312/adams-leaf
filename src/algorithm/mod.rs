mod base;
mod aco;
mod ro;
mod spf;
mod ants;
mod aco_routing;

pub use ants::AdamsAnt;
pub use ro::RO;
pub use spf::SPF;


use crate::utils::stream::{AVBFlow, FlowID, TSNFlow};
use crate::component::RoutingCost;

pub trait RoutingAlgo {
    fn add_flows(&mut self, tsns: Vec<TSNFlow>, avbs: Vec<AVBFlow>);
    fn del_flows(&mut self, tsns: Vec<TSNFlow>, avbs: Vec<AVBFlow>);
    fn get_rerouted_flows(&self) -> &Vec<FlowID>;
    fn get_route(&self, id: FlowID) -> &Vec<usize>;
    fn show_results(&self);
    fn get_last_compute_time(&self) -> u128;
    fn get_cost(&self) -> RoutingCost;
}
