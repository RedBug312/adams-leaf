use crate::network::Network;
use itertools::Itertools;
use std::collections::HashMap;
use super::dijkstra::Dijkstra;
use super::heap::MyMinHeap;


type Path = Vec<usize>;


#[derive(Default)]
pub struct Yens {
    path: HashMap<(usize, usize), Vec<Path>>,
    k: usize,
    dijkstra: Dijkstra,
}

impl Yens {
    pub fn new(graph: &Network, max_k: usize) -> Self {
        let mut yens = Self::default();
        yens.compute(graph, max_k);
        yens
    }
    pub fn compute(&mut self, graph: &Network, k: usize) {
        self.k = k;
        self.dijkstra.compute(graph);
        // compute_pair on all end devices pair takes 20 sec on test case
        for (&src, &dst) in graph.end_devices.iter().tuple_combinations() {
            self.compute_pair(graph, src, dst, 10);
            self.compute_pair(graph, dst, src, 10);
        }
    }
    pub fn compute_pair(&mut self, graph: &Network, src: usize, dst: usize, k: usize) {
        if self.path.contains_key(&(src, dst)) { return }
        debug_assert!(src != dst);

        self.dijkstra.compute_pair(graph, src);
        // TODO dump panic message for not connected
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
                dijkstra.compute_pair(graph, spur_node);

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
    pub fn k_shortest_paths(&self, src: usize, dst: usize) -> &Vec<Path> {
        static EMPTY: Vec<Path> = vec![];
        let ends = (src, dst);
        self.path.get(&ends).unwrap_or(&EMPTY)
    }
}



#[cfg(test)]
mod tests {
    use crate::network::Network;
    use itertools::Itertools;
    use super::Yens;
    #[test]
    #[ignore]
    fn it_runs_yens_on_pairs() {
        let mut network = Network::default();
        network.add_nodes(6, 93);  // 0..=5 + 99, 6..=98
        network.add_nodes(1, 0);
        network.add_edges(vec![
            (0, 1, 10.0), (1, 2, 20.0), (0, 2, 02.0), (1, 4, 10.0),
            (1, 3, 15.0), (2, 3, 10.0), (2, 4, 10.0)
        ]);
        let more_edges = (4..100).tuple_combinations()
            .map(|(src, dst)| (src, dst, src as f64 * dst as f64))
            .collect();
        network.add_edges(more_edges);

        let mut yens = Yens::default();
        yens.compute_pair(&network, 0, 2, 10);
        yens.compute_pair(&network, 0, 5, 10);
        yens.compute_pair(&network, 0, 99, 10);
        let yens_kth = |src, dst, kth| yens.kth_shortest_path(src, dst, kth);

        assert_eq!(yens.count_shortest_paths(0, 2), 4);
        assert_eq!(yens_kth(0, 2, 0), Some(&vec![0, 1, 2]));
        assert_eq!(yens_kth(0, 2, 1), Some(&vec![0, 1, 3, 2]));
        assert_eq!(yens_kth(0, 2, 2), Some(&vec![0, 1, 4, 2]));

        assert_eq!(yens.count_shortest_paths(0, 5), 10);
        assert_eq!(yens_kth(0, 5, 0), Some(&vec![0, 1, 4, 99, 5]));
        assert_eq!(yens_kth(0, 5, 1), Some(&vec![0, 1, 4, 98, 5]));
        assert_eq!(yens_kth(0, 5, 2), Some(&vec![0, 1, 4, 97, 5]));
        assert_eq!(yens_kth(0, 5, 3), Some(&vec![0, 1, 4, 99, 98, 5]));
        assert_eq!(yens_kth(0, 5, 4), Some(&vec![0, 1, 4, 98, 99, 5]));

        assert_eq!(yens.count_shortest_paths(0, 99), 10);
        assert_eq!(yens_kth(0, 99, 0), Some(&vec![0, 1, 4, 99]));
        assert_eq!(yens_kth(0, 99, 1), Some(&vec![0, 1, 4, 98, 99]));
        assert_eq!(yens_kth(0, 99, 2), Some(&vec![0, 1, 4, 97, 99]));
    }
}
