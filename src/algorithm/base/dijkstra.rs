use std::collections::{HashMap, HashSet};
use std::f64::INFINITY as INF;

use super::heap::MyMinHeap;
use crate::network::Network;

pub type Path = Vec<usize>;


#[derive(Default)]
pub struct Dijkstra {
    dist: HashMap<(usize, usize), f64>,
    pred: HashMap<(usize, usize), usize>,
    ignore_nodes: HashSet<usize>,
    ignore_edges: HashSet<(usize, usize)>,
}


impl Dijkstra {
    pub fn compute(&mut self, graph: &Network) {
        for &root in graph.end_devices.iter() {
            self.compute_once(graph, root);
        }
    }
    pub fn compute_once(&mut self, graph: &Network, r: usize) {
        if self.dist.contains_key(&(r, r)) { return }
        let mut heap = MyMinHeap::new();
        let mut seen = HashMap::new();

        seen.insert(r, 0.0);
        heap.push(r, 0.0.into());

        // 從優先權佇列中移除，並塞進最終 dist map
        while let Some((v, rv_dist)) = heap.pop() {
            match self.dist.contains_key(&(r, v)) {
                true  => { continue; },
                false => { self.dist.insert((r, v), rv_dist.into()); },
            }
            for &u in graph.node(v).neighbors.iter() {
                if self.ignore_nodes.contains(&u)
                    || self.ignore_edges.contains(&(v, u)) { continue; }

                let cost = graph.duration_on(&[v, u], 1.0);
                let ru_dist = self.dist.get(&(r, v)).unwrap() + cost;

                if self.dist.contains_key(&(r, u))
                    || ru_dist >= *seen.get(&u).unwrap_or(&INF){ continue; }

                self.pred.insert((r, u), v);
                seen.insert(u, ru_dist);
                match heap.get(&u) {
                    Some(_) => { heap.change_priority(&u, ru_dist.into()); },
                    None    => { heap.push(u, ru_dist.into()); },
                }
            }
        }
    }
    pub fn shortest_path(&self, src: usize, dst: usize) -> Option<Path> {
        match self.dist.contains_key(&(src, dst)) {
            true  => Some(self._recursive_backtrace(src, dst)),
            false => None,
        }
    }
    pub fn ignore(&mut self, nodes: HashSet<usize>, edges: HashSet<(usize, usize)>) {
        self.ignore_nodes = nodes;
        self.ignore_edges = edges;
    }
    fn _recursive_backtrace(&self, src: usize, dst: usize) -> Path {
        if src == dst {
            vec![src]
        } else {
            let &pred = self.pred.get(&(src, dst))
                            .expect("Error when backtrace path");
            let mut path = self._recursive_backtrace(src, pred);
            path.push(dst);
            path
        }
    }
}



#[cfg(test)]
mod test {
    use super::Dijkstra;
    use crate::network::Network;
    #[test]
    fn test_dijkstra_case1() {
        let mut graph = Network::default();
        graph.add_nodes(3, 0);
        graph.add_edges(vec![
            (0, 1, 10.0), (0, 1, 10.0), (1, 2, 20.0), (0, 2, 02.0),
        ]);
        let mut dijkstra = Dijkstra::default();
        dijkstra.compute(&graph);
        assert_eq!(dijkstra.shortest_path(0, 2), Some(vec![0, 1, 2]));
    }
    #[test]
    fn test_dijkstra_case2() {
        let mut graph = Network::default();
        graph.add_nodes(6, 0);
        graph.add_edges(vec![
            (0, 1, 10.0), (1, 2, 20.0), (0, 2, 02.0), (1, 3, 10.0),
            (0, 3, 03.0), (3, 4, 03.0),
        ]);
        let mut dijkstra = Dijkstra::default();
        dijkstra.compute(&graph);
        assert_eq!(dijkstra.shortest_path(0, 4), Some(vec![0, 1, 3, 4]));
        assert_eq!(dijkstra.shortest_path(2, 4), Some(vec![2, 1, 3, 4]));
        assert_eq!(dijkstra.shortest_path(3, 3), Some(vec![3]));
        assert_eq!(dijkstra.shortest_path(0, 5), None);
    }
}

