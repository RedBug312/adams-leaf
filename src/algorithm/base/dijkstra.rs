use std::collections::HashSet;
use super::heap::MyMinHeap;
use crate::network::{EdgeIndex, NodeIndex, Network};

pub type Path = Vec<EdgeIndex>;

#[derive(Default)]
pub struct Dijkstra {
    dist: Vec<Vec<f64>>,
    pred: Vec<Vec<(NodeIndex, EdgeIndex)>>,
    ignore_nodes: HashSet<NodeIndex>,
    ignore_edges: HashSet<EdgeIndex>,
}

impl Dijkstra {
    pub fn new(graph: &Network) -> Self {
        let unknown = (usize::MAX.into(), usize::MAX.into());
        let node_count = graph.node_count();
        let dist = vec![vec![f64::INFINITY; node_count]; node_count];
        let pred = vec![vec![unknown; node_count]; node_count];
        Dijkstra { dist, pred, ..Default::default() }
    }
    #[allow(dead_code)]
    pub fn compute(&mut self, graph: &Network) {
        for &root in &graph.end_devices {
            self.compute_root(graph, root);
        }
    }
    pub fn compute_root(&mut self, graph: &Network, r: NodeIndex) {
        let mut heap = MyMinHeap::new();
        let mut seen = vec![f64::INFINITY; graph.node_count()];
        let dist = &mut self.dist[r.index()];
        let pred = &mut self.pred[r.index()];

        if dist[r.index()] == 0.0 { return; }
        seen[r.index()] = 0.0;
        heap.push(r, 0.0.into());

        // 從優先權佇列中移除，並塞進最終 dist map
        while let Some((v, rv_dist)) = heap.pop() {
            match dist[v.index()] {
                #[allow(illegal_floating_point_literal_pattern)]
                f64::INFINITY => { dist[v.index()] = rv_dist.into(); },
                _ => { continue; },
            }
            for vu in graph.outgoings(v) {
                let u = graph.endpoints(vu).1;
                if self.ignore_nodes.contains(&u)
                    || self.ignore_edges.contains(&vu) { continue; }

                let cost = graph.duration_on(vu, 1.0);
                let ru_dist = dist[v.index()] + cost;

                if dist[u.index()] != f64::INFINITY
                    || ru_dist >= seen[u.index()] { continue; }

                pred[u.index()] = (v, vu);
                seen[u.index()] = ru_dist;
                match heap.get(&u) {
                    Some(_) => { heap.change_priority(&u, ru_dist.into()); },
                    None    => { heap.push(u, ru_dist.into()); },
                }
            }
        }
    }
    pub fn shortest_path(&self, src: NodeIndex, dst: NodeIndex) -> Option<Path> {
        let dist = &self.dist[src.index()];
        match dist[dst.index()] {
            #[allow(illegal_floating_point_literal_pattern)]
            f64::INFINITY => None,
            _ => Some(self._recursive_backtrace(src, dst)),
        }
    }
    pub fn ignore(&mut self, nodes: HashSet<NodeIndex>, edges: HashSet<EdgeIndex>) {
        self.ignore_nodes = nodes;
        self.ignore_edges = edges;
    }
    fn _recursive_backtrace(&self, src: NodeIndex, dst: NodeIndex) -> Path {
        if src == dst { return vec![]; }
        let (pred, last) = self.pred[src.index()][dst.index()];
        let mut path = self._recursive_backtrace(src, pred);
        path.push(last);
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_runs_dijkstra_on_case1() {
        let mut network = Network::new();
        network.add_nodes(3, 0);
        network.add_edges(vec![
            (0, 1, 10.0), (0, 1, 10.0), (1, 2, 20.0), (0, 2, 02.0),
        ]);
        let mut dijkstra = Dijkstra::new(&network);
        dijkstra.compute(&network);
        let dijk = |src: usize, dst: usize| {
            dijkstra.shortest_path(src.into(), dst.into())
                .map(|path| network.node_sequence(&path))
        };
        assert_eq!(dijk(0, 2), Some(vec![0, 1, 2]));
    }

    #[test]
    fn it_runs_dijkstra_on_case2() {
        let mut network = Network::new();
        network.add_nodes(6, 0);
        network.add_edges(vec![
            (0, 1, 10.0), (1, 2, 20.0), (0, 2, 02.0), (1, 3, 10.0),
            (0, 3, 03.0), (3, 4, 03.0),
        ]);
        let mut dijkstra = Dijkstra::new(&network);
        dijkstra.compute(&network);
        let dijk = |src: usize, dst: usize| {
            dijkstra.shortest_path(src.into(), dst.into())
                .map(|path| network.node_sequence(&path))
        };
        assert_eq!(dijk(0, 4), Some(vec![0, 1, 3, 4]));
        assert_eq!(dijk(2, 4), Some(vec![2, 1, 3, 4]));
        assert_eq!(dijk(3, 3), Some(vec![]));
        assert_eq!(dijk(0, 5), None);
    }
}
