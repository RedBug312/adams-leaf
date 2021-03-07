use crate::network::{Graph, OnOffGraph};
use std::collections::HashMap;

pub struct Node {
    pub neighbors: Vec<usize>,
    is_switch: bool,
    edges: HashMap<usize, (f64, bool)>,
    exist: bool,
    active: bool,
}
impl Clone for Node {
    fn clone(&self) -> Self {
        let mut edges: HashMap<usize, (f64, bool)> = HashMap::new();
        for (&id, &edge) in self.edges.iter() {
            edges.insert(id, edge);
        }
        return Node {
            neighbors: self.neighbors.clone(),
            is_switch: self.is_switch,
            exist: self.exist,
            active: self.active,
            edges,
        };
    }
}

#[derive(Clone)]
pub struct Edge {
    pub ends: (usize, usize),
    pub bandwidth: f64,
}

impl Edge {
    pub fn new(ends: (usize, usize), bandwidth: f64) -> Self {
        Edge { ends, bandwidth }
    }
}

#[derive(Clone, Default)]
pub struct StreamAwareGraph {
    pub nodes: Vec<Node>,
    pub edges: HashMap<(usize, usize), Edge>,
    node_cnt: usize,
    edge_cnt: usize,
    cur_edge_id: usize,
    inactive_edges: Vec<(usize, usize)>,
    inactive_nodes: Vec<usize>,
    pub(super) edge_info: HashMap<(usize, usize), (usize, f64)>,
    pub end_devices: Vec<usize>,
}
impl StreamAwareGraph {
    fn _add_node(&mut self, cnt: Option<usize>, is_switch: bool) -> Vec<usize> {
        let cnt = {
            if let Some(_cnt) = cnt {
                _cnt
            } else {
                1
            }
        };
        let mut v: Vec<usize> = vec![];
        for _ in 0..cnt {
            let id = self.nodes.len();
            self.node_cnt += 1;
            let node = Node {
                neighbors: vec![],
                is_switch,
                exist: true,
                active: true,
                edges: HashMap::new(),
            };
            self.nodes.push(node);
            v.push(id);
            self.end_devices.push(id);
        }
        return v;
    }
    pub fn node(&self, id: usize) -> &Node {
        self.nodes.get(id)
            .expect("node not found")
    }
    pub fn edge(&self, ends: &[usize]) -> &Edge {
        debug_assert!(ends.len() == 2);
        debug_assert!(ends[0] != ends[1]);
        self.edges.get(&(ends[0], ends[1]))
            .expect("edge not found")
    }
    pub fn duration_on(&self, ends: &[usize], size: f64) -> f64 {
        size / self.edge(ends).bandwidth
    }
    pub fn duration_along(&self, path: &Vec<usize>, size: f64) -> f64 {
        path.windows(2)
            .map(|ends| self.duration_on(ends, size))
            .sum()
    }
    fn _check_exist(&self, id: usize) -> bool {
        return id < self.nodes.len() && self.nodes[id].exist;
    }
    fn _add_single_edge(&mut self, id: usize, node_pair: (usize, usize), bandwidth: f64) {
        self.nodes[node_pair.0]
            .edges
            .insert(node_pair.1, (bandwidth, true));
        self.edge_info.insert(node_pair, (id, bandwidth));

        let (end0, end1) = node_pair;
        self.nodes[end0].neighbors.push(end1);
        self.nodes[end1].neighbors.push(end0);

        let ends = (end0, end1);
        self.edges.insert(ends, Edge::new(ends, bandwidth));
        let ends = (end1, end0);
        self.edges.insert(ends, Edge::new(ends, bandwidth));
    }
    fn _del_single_edge(&mut self, id_pair: (usize, usize)) -> Result<f64, String> {
        if let Some(e) = self.nodes[id_pair.0].edges.remove(&id_pair.1) {
            return Ok(e.0);
        } else {
            return Err("刪除邊時發現邊不存在".to_owned());
        }
    }

    fn _change_edge_active(&mut self, id_pair: (usize, usize), active: bool) -> Result<(), String> {
        if let Some(e) = self.nodes[id_pair.0].edges.get_mut(&id_pair.1) {
            e.1 = active;
            return Ok(());
        } else {
            return Err("修改邊的活性時發現邊不存在".to_owned());
        }
    }
    fn _change_node_active(&mut self, id: usize, active: bool) -> Result<(), String> {
        if self._check_exist(id) {
            self.nodes[id].active = active;
            return Ok(());
        } else {
            return Err("修改節點的活性時發現節點不存在".to_owned());
        }
    }
    pub fn new() -> Self {
        StreamAwareGraph {
            nodes: vec![],
            edges: Default::default(),
            node_cnt: 0,
            edge_cnt: 0,
            cur_edge_id: 0,
            inactive_edges: vec![],
            inactive_nodes: vec![],
            edge_info: HashMap::new(),
            end_devices: vec![],
        }
    }
    pub fn get_links_id_bandwidth(&self, route: &Vec<usize>) -> Vec<((usize, usize), f64)> {
        let mut vec = vec![];
        for i in 0..route.len() - 1 {
            let ends = (route[i], route[i + 1]);
            if let Some(tuple) = self.edge_info.get(&ends) {
                vec.push((ends, tuple.1));
            } else {
                panic!("get_link_ids: 不連通的路徑");
            }
        }
        vec
    }
}
impl Graph<usize> for StreamAwareGraph {
    fn add_host(&mut self, cnt: Option<usize>) -> Vec<usize> {
        return self._add_node(cnt, false);
    }
    fn add_switch(&mut self, cnt: Option<usize>) -> Vec<usize> {
        return self._add_node(cnt, true);
    }
    fn get_edge_cnt(&self) -> usize {
        return self.edge_cnt;
    }
    fn get_node_cnt(&self) -> usize {
        return self.node_cnt;
    }
    fn add_edge(
        &mut self,
        id_pair: (usize, usize),
        bandwidth: f64,
    ) -> Result<(usize, usize), String> {
        if self._check_exist(id_pair.0) && self._check_exist(id_pair.1) {
            let edge_id = self.cur_edge_id;
            self._add_single_edge(edge_id, id_pair, bandwidth);
            self._add_single_edge(edge_id + 1, (id_pair.1, id_pair.0), bandwidth);
            self.edge_cnt += 2;
            self.cur_edge_id += 2;
            return Ok((edge_id, edge_id + 1));
        } else {
            return Err("加入邊時發現節點不存在".to_owned());
        }
    }
    fn del_edge(&mut self, id_pair: (usize, usize)) -> Result<f64, String> {
        if self._check_exist(id_pair.0) && self._check_exist(id_pair.1) {
            self._del_single_edge(id_pair)?;
            self.edge_cnt -= 1;
            return self._del_single_edge((id_pair.1, id_pair.0));
        } else {
            return Err("刪除邊時發現節點不存在".to_owned());
        }
    }
    fn del_node(&mut self, id: usize) -> Result<(), String> {
        if self._check_exist(id) {
            let _self = self as *mut Self;
            let edges = &self.nodes[id].edges;
            for (&next_id, _edge) in edges.iter() {
                unsafe {
                    if let Err(msg) = (*_self).del_edge((next_id, id)) {
                        panic!(msg);
                    }
                }
            }
            self.nodes[id].exist = false;
            self.node_cnt -= 1;
            return Ok(());
        } else {
            return Err("找不到欲刪除的節點".to_owned());
        }
    }
    fn foreach_edge(&self, id: usize, mut callback: impl FnMut(usize, f64) -> ()) {
        let node = &self.nodes[id];
        for (&id, &(bandwidth, active)) in node.edges.iter() {
            let node = &self.nodes[id];
            if active && node.exist && node.active {
                callback(id, bandwidth);
            }
        }
    }
    fn foreach_node(&self, mut callback: impl FnMut(usize, bool) -> ()) {
        for (id, node) in self.nodes.iter().enumerate() {
            if node.exist && node.active {
                callback(id, node.is_switch);
            }
        }
    }
    fn get_dist(&self, path: &Vec<usize>) -> f64 {
        let mut dist = 0.0;
        for i in 0..path.len() - 1 {
            let (cur, next) = (path[i], path[i + 1]);
            if let Some((bandwidth, _)) = self.nodes[cur].edges.get(&next) {
                dist += 1.0 / bandwidth;
            } else {
                panic!("嘗試對一條走不通的路徑取距離");
            }
        }
        dist
    }
}
impl OnOffGraph<usize> for StreamAwareGraph {
    #[allow(unused_must_use)]
    fn inactivate_edge(&mut self, id_pair: (usize, usize)) -> Result<(), String> {
        if self._check_exist(id_pair.0) && self._check_exist(id_pair.1) {
            self._change_edge_active(id_pair, false)?;
            self._change_edge_active((id_pair.1, id_pair.0), false);
            self.inactive_edges.push(id_pair);
            return Ok(());
        } else {
            return Err("修改邊的活性時發現節點不存在".to_owned());
        }
    }
    fn inactivate_node(&mut self, id: usize) -> Result<(), String> {
        self._change_node_active(id, false)?;
        self.inactive_nodes.push(id);
        return Ok(());
    }
    #[allow(unused_must_use)]
    fn reset(&mut self) {
        let _self = self as *mut Self;
        for pair in self.inactive_edges.iter() {
            unsafe {
                (*_self)._change_edge_active(*pair, true);
                (*_self)._change_edge_active((pair.1, pair.0), true);
            }
        }
        for id in self.inactive_nodes.iter() {
            unsafe {
                (*_self)._change_node_active(*id, true);
            }
        }
        self.inactive_edges.clear();
        self.inactive_nodes.clear();
    }
}
