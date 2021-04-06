use std::hash::Hash;

pub trait Graph<K: Hash + Eq>: Clone {
    fn add_host(&mut self, cnt: Option<usize>) -> Vec<K>;
    fn add_switch(&mut self, cnt: Option<usize>) -> Vec<K>;
    fn add_edge(&mut self, id_pair: (K, K), bandwidth: f64) -> Result<(K, K), String>;
    fn del_edge(&mut self, id_pair: (K, K)) -> Result<f64, String>;
    fn del_node(&mut self, id: K) -> Result<(), String>;
    fn foreach_edge(&self, id: K, callback: impl FnMut(K, f64) -> ());
    fn foreach_node(&self, callback: impl FnMut(K, bool) -> ());
    fn get_dist(&self, path: &Vec<K>) -> f64;
    /// __注意：一個邊會被算兩次，來回各一次__
    fn get_edge_cnt(&self) -> usize;
    fn get_node_cnt(&self) -> usize;
}

pub trait OnOffGraph<K: Hash + Eq>: Graph<K> {
    fn inactivate_edge(&mut self, id_pair: (K, K)) -> Result<(), String>;
    fn inactivate_node(&mut self, id: K) -> Result<(), String>;
    fn reset(&mut self);
}
