use std::iter;
use hashbrown::HashMap;


#[derive(Clone, Debug)]
pub enum Device {
    EndDevice,
    Bridge,
}
#[derive(Clone, Debug)]
pub struct Node {
    pub device: Device,
    pub neighbors: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct Edge {
    pub ends: (usize, usize),
    pub bandwidth: f64,
}


impl Node {
    pub fn new(device: Device) -> Self {
        Self { device, neighbors: vec![] }
    }
}
impl Edge {
    pub fn new(ends: (usize, usize), bandwidth: f64) -> Self {
        Edge { ends, bandwidth }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Network {
    pub nodes: Vec<Node>,
    pub edges: HashMap<(usize, usize), Edge>,
    pub end_devices: Vec<usize>,
}

impl Network {
    pub fn new() -> Self {
        Self { ..Default::default() }
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
    pub fn add_nodes(&mut self, end_device_count: usize, bridge_count: usize) {
        let new_devices = self.nodes.len()..self.nodes.len()+end_device_count;
        self.end_devices.extend(new_devices);
        let devices = iter::repeat_with(|| Node::new(Device::EndDevice))
            .take(end_device_count);
        let bridges = iter::repeat_with(|| Node::new(Device::Bridge))
            .take(bridge_count);
        self.nodes.extend(devices);
        self.nodes.extend(bridges);
    }
    pub fn add_edges(&mut self, edges: Vec<(usize, usize, f64)>) {
        for (end0, end1, bandwidth) in edges.into_iter() {
            self.nodes[end0].neighbors.push(end1);
            self.nodes[end1].neighbors.push(end0);
            debug_assert!(end0 != end1);
            let ends = (end0, end1);
            self.edges.insert(ends, Edge::new(ends, bandwidth));
            let ends = (end1, end0);
            self.edges.insert(ends, Edge::new(ends, bandwidth));
        }
    }
    pub fn duration_on(&self, ends: &[usize], size: f64) -> f64 {
        size / self.edge(ends).bandwidth
    }
    pub fn duration_along(&self, path: &Vec<usize>, size: f64) -> f64 {
        path.windows(2)
            .map(|ends| self.duration_on(ends, 1f64))
            .sum::<f64>() * size
    }
    // TODO remove this function
    pub fn get_links_id_bandwidth(&self, path: &Vec<usize>) -> Vec<((usize, usize), f64)> {
        path.windows(2)
            .map(|ends| self.edge(ends))
            .map(|edge| (edge.ends, edge.bandwidth))
            .collect()
    }
}
