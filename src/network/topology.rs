use std::iter;

#[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct NodeIndex(usize);

impl NodeIndex {
    fn new(ix: usize) -> Self {
        NodeIndex(ix)
    }
    pub fn index(self) -> usize {
        self.0
    }
}

impl From<usize> for NodeIndex {
    fn from(ix: usize) -> Self {
        NodeIndex::new(ix)
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct EdgeIndex(usize);

impl EdgeIndex {
    fn new(ix: usize) -> Self {
        EdgeIndex(ix)
    }
    pub fn index(self) -> usize {
        self.0
    }
}

impl From<usize> for EdgeIndex {
    fn from(ix: usize) -> Self {
        EdgeIndex::new(ix)
    }
}

#[derive(Clone, Debug)]
pub enum Device {
    EndDevice,
    Bridge,
}

#[derive(Clone, Debug)]
pub struct Node {
    edges: Vec<EdgeIndex>,
    device: Device,
}

#[derive(Clone, Debug)]
pub struct Edge {
    ends: (NodeIndex, NodeIndex),
    bandwidth: f64,
}

impl Node {
    pub fn new(device: Device) -> Self {
        Self { device, edges: vec![] }
    }
}
impl Edge {
    pub fn new(ends: (NodeIndex, NodeIndex), bandwidth: f64) -> Self {
        Edge { ends, bandwidth }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Network {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub end_devices: Vec<NodeIndex>,
}

impl Network {
    pub fn new() -> Self {
        Self { ..Default::default() }
    }
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
    pub fn endpoints(&self, edge: EdgeIndex) -> &(NodeIndex, NodeIndex) {
        debug_assert!(edge.index() < self.edges.len());
        &self.edges[edge.index()].ends
    }
    pub fn outgoings(&self, node: NodeIndex)
        -> impl Iterator<Item=EdgeIndex> + '_ {
        debug_assert!(node.index() < self.nodes.len());
        self.nodes[node.index()].edges.iter().cloned()
    }
    pub fn neighbors(&self, node: NodeIndex)
        -> impl Iterator<Item=NodeIndex> + '_ {
        debug_assert!(node.index() < self.nodes.len());
        self.nodes[node.index()].edges.iter()
            .map(move |&e| self.edges[e.index()].ends.1)
    }
    pub fn add_nodes(&mut self, end_device_count: usize, bridge_count: usize) {
        let node_count = self.nodes.len();
        let new_devices = (node_count..node_count + end_device_count)
            .map(NodeIndex::new);
        self.end_devices.extend(new_devices);
        let devices = iter::repeat_with(|| Node::new(Device::EndDevice))
            .take(end_device_count);
        let bridges = iter::repeat_with(|| Node::new(Device::Bridge))
            .take(bridge_count);
        self.nodes.extend(devices);
        self.nodes.extend(bridges);
    }
    pub fn add_edges(&mut self, edges: Vec<(usize, usize, f64)>) {
        for (end0, end1, bandwidth) in edges {
            debug_assert!(end0 != end1);
            let ends = (end0.into(), end1.into());
            self.nodes[end0].edges.push(EdgeIndex::new(self.edges.len()));
            self.edges.push(Edge::new(ends, bandwidth));
            let ends = (end1.into(), end0.into());
            self.nodes[end1].edges.push(EdgeIndex::new(self.edges.len()));
            self.edges.push(Edge::new(ends, bandwidth));
        }
    }
    pub fn duration_on(&self, edge: EdgeIndex, size: u32) -> f64 {
        debug_assert!(edge.index() < self.edges.len());
        size as f64 / self.edges[edge.index()].bandwidth
    }
    pub fn duration_along(&self, path: &[EdgeIndex], size: u32) -> f64 {
        path.iter()
            .map(|&e| self.duration_on(e, 1))
            .sum::<f64>() * size as f64
    }
    pub fn node_sequence(&self, path: &[EdgeIndex]) -> Vec<usize> {
        if path.is_empty() {
            return vec![];
        }
        let head = self.endpoints(path[0]).0.index();
        let tail = path.iter()
            .map(|&e| self.endpoints(e).1.index());
        iter::once(head).chain(tail).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_lookups_edge_ends() {
        let mut network = Network::default();
        network.add_nodes(3, 0);
        network.add_edges(vec![(0, 1, 10.0), (1, 2, 20.0), (0, 2, 02.0)]);
        assert_eq!(network.endpoints(0.into()), &(0.into(), 1.into()));
        assert_eq!(network.endpoints(1.into()), &(1.into(), 0.into()));
        assert_eq!(network.endpoints(2.into()), &(1.into(), 2.into()));
        assert_eq!(network.endpoints(3.into()), &(2.into(), 1.into()));
    }
}
