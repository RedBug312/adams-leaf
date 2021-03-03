extern crate rand;
// use rand::{Rng, ThreadRng};

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use super::dijkstra::Dijkstra;
use super::heap::MyMinHeap;
use crate::network::StreamAwareGraph;

type Path = Vec<usize>;

#[derive(Default)]
pub struct YensAlgo {
    path: HashMap<(usize, usize), Vec<Path>>,
    k: usize,
    dijkstra: Dijkstra,
}

impl YensAlgo {
    pub fn compute(&mut self, graph: &StreamAwareGraph, k: usize) {
        self.k = k;
        self.dijkstra.compute(graph);
        // compute_once on all end devices pair takes 20 sec on test case
        for &src in graph.end_devices.iter() {
            for &dst in graph.end_devices.iter().filter(|&&node| node != src) {
                self.compute_once(graph, src, dst, 10);
            }
        }
    }
    pub fn compute_once(&mut self, graph: &StreamAwareGraph, src: usize, dst: usize, k: usize) {
        if self.path.contains_key(&(src, dst)) { return }
        debug_assert!(src != dst);

        self.dijkstra.compute_once(graph, src);
        let shortest = self.dijkstra.shortest_path(src, dst).unwrap();
        let mut list_a = vec![shortest];
        let mut heap_b = MyMinHeap::new();

        for k in 0..=k-2 {
            let prev_path_len = list_a[k].len();
            for i in 0..=(prev_path_len - 2) {
                let spur_node = list_a[k][i];
                let mut root_path = list_a[k][..=i].to_vec();

                // For example, if search for 4th shortest path with spur-node (2)
                // We should ignore edges (2)───(3), (2)───(5) and node (1)
                //
                // (1)───(2)───(3)───(4)  1st
                //  │     └────(5)───(4)  2nd
                //  └────(7)───(8)───(4)  3rd

                let ignored_edges = list_a.iter()
                    .filter(|path| path.len() > i && path[..=i] == list_a[k][..=i])
                    .map(|path| (path[i], path[i+1]))
                    .collect();
                let ignored_nodes = list_a[k][..i].iter()
                    .cloned()
                    .collect();

                let mut dijkstra = Dijkstra::default();
                dijkstra.ignore(ignored_nodes, ignored_edges);
                dijkstra.compute_once(graph, spur_node);

                match dijkstra.shortest_path(spur_node, dst) {
                    Some(spur_path) => {
                        root_path.pop();
                        root_path.extend(spur_path);
                        let total_path = root_path;
                        let total_dist = graph.duration_along(&total_path, 1.0);
                        heap_b.push(total_path, total_dist.into());
                    }
                    None => continue,  // spur-dst exists no more paths
                }
            }
            match heap_b.pop() {
                Some(path) => list_a.push(path.0),
                None       => break,  // src-dst exists no more paths
            }
        }
        self.path.insert((src, dst), list_a);
    }
    pub fn kth_shortest_path(&self, src: usize, dst: usize, k: usize) -> Option<&Path> {
        self.path.get(&(src, dst))
            .and_then(|paths| paths.get(k))
    }
    pub fn count_shortest_paths(&self, src: usize, dst: usize) -> usize {
        self.path.get(&(src, dst))
            .map(|paths| paths.len())
            .unwrap_or(0)
    }
}



#[cfg(test)]
mod test {
    use super::Yens;
    use crate::graph::StreamAwareGraph;
    #[test]
    #[ignore]
    fn test_yens_compute_once() {
        let mut graph = StreamAwareGraph::default();
        graph.add_nodes(6, 93);  // 0..=5 + 99, 6..=98
        graph.add_nodes(1, 0);
        graph.add_edges(vec![
            (0, 1, 10.0), (1, 2, 20.0), (0, 2, 02.0), (1, 4, 10.0),
            (1, 3, 15.0), (2, 3, 10.0), (2, 4, 10.0)
        ]);
        for src in 4..100 {
            for dst in src+1..100 {
                graph.add_edges(vec![(src, dst, (src * dst) as f64)]);
            }
        }
        let mut yens = Yens::default();

        yens.compute_once(&graph, 0, 2, 10);
        assert_eq!(yens.count_shortest_paths(0, 2), 4);
        assert_eq!(yens.kth_shortest_path(0, 2, 0), Some(&vec![0, 1, 2]));
        assert_eq!(yens.kth_shortest_path(0, 2, 1), Some(&vec![0, 1, 3, 2]));
        assert_eq!(yens.kth_shortest_path(0, 2, 2), Some(&vec![0, 1, 4, 2]));

        yens.compute_once(&graph, 0, 99, 10);
        assert_eq!(yens.count_shortest_paths(0, 99), 10);
        assert_eq!(yens.kth_shortest_path(0, 99, 0), Some(&vec![0, 1, 4, 99]));
        assert_eq!(yens.kth_shortest_path(0, 99, 1), Some(&vec![0, 1, 4, 98, 99]));
        assert_eq!(yens.kth_shortest_path(0, 99, 2), Some(&vec![0, 1, 4, 97, 99]));

        yens.compute_once(&graph, 0, 5, 10);
        assert_eq!(yens.count_shortest_paths(0, 5), 10);
        assert_eq!(yens.kth_shortest_path(0, 5, 0), Some(&vec![0, 1, 4, 99, 5]));
        assert_eq!(yens.kth_shortest_path(0, 5, 1), Some(&vec![0, 1, 4, 98, 5]));
        assert_eq!(yens.kth_shortest_path(0, 5, 2), Some(&vec![0, 1, 4, 97, 5]));
        assert_eq!(yens.kth_shortest_path(0, 5, 3), Some(&vec![0, 1, 4, 99, 98, 5]));
        assert_eq!(yens.kth_shortest_path(0, 5, 4), Some(&vec![0, 1, 4, 98, 99, 5]));
    }
}
