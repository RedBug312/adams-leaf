use crate::network::{EdgeIndex, NodeIndex, Network};
use itertools::Itertools;
use super::dijkstra::Dijkstra;
use super::heap::MyMinHeap;

type Path = Vec<EdgeIndex>;

#[derive(Default)]
pub struct Yens {
    dijkstra: Dijkstra,
    paths: Vec<Vec<Vec<Path>>>,
    k: usize,
}

impl Yens {
    pub fn new(graph: &Network, k: usize) -> Self {
        let node_count = graph.node_count();
        let dijkstra = Dijkstra::new(graph);
        let paths = vec![vec![Vec::with_capacity(k); node_count]; node_count];
        Yens { dijkstra, paths, k, ..Default::default() }
    }
    pub fn compute(&mut self, graph: &Network) {
        // compute_pair on all end devices pair takes 20 sec on test case
        for (&src, &dst) in graph.end_devices.iter().tuple_combinations() {
            self.compute_pair(graph, src.into(), dst.into());
            self.compute_pair(graph, dst.into(), src.into());
        }
    }
    pub fn compute_pair(&mut self, graph: &Network, src: NodeIndex, dst: NodeIndex) {
        debug_assert!(src != dst);
        if self.paths[src.index()][dst.index()].len() > 0 { return; }

        self.dijkstra.compute_root(graph, src);
        // TODO dump panic message for not connected
        let shortest = self.dijkstra.shortest_path(src, dst).unwrap();
        let k = self.k;
        let mut list_a = vec![shortest];
        let mut heap_b = MyMinHeap::new();

        for k in 0..k-1 {
            let prev_path_len = list_a[k].len();
            for i in 0..=(prev_path_len-1) {
                let spur_edge = list_a[k][i];  // where endpoints.0 is the spur-node
                let spur_node = graph.endpoints(spur_edge).0;
                let mut root_path = list_a[k][..=i].to_vec();
                // println!("{:?}", (k, i));
                // println!("{:?}", node_sequence(graph, &root_path));

                // For example, if search for 4th shortest path with spur-node (2)
                // We should ignore edges (2)───(3), (2)───(5) and node (1)
                //
                // (1)───(2)───(3)───(4)  1st
                //  │     └────(5)───(4)  2nd
                //  └────(7)───(8)───(4)  3rd

                let ignored_edges = list_a.iter()
                    .filter(|path| i < path.len() && path[..i] == list_a[k][..i])
                    .map(|path| path[i])
                    .collect();
                let ignored_nodes = list_a[k][..i].iter()
                    .map(|&edge| graph.endpoints(edge).0)
                    .collect();

                let mut dijkstra = Dijkstra::new(&graph);
                dijkstra.ignore(ignored_nodes, ignored_edges);
                dijkstra.compute_root(graph, spur_node);

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
        self.paths[src.index()][dst.index()] = list_a;
    }
    pub fn kth_shortest_path(&self, src: NodeIndex, dst: NodeIndex, k: usize) -> Option<&Path> {
        self.paths[src.index()][dst.index()].get(k)
    }
    pub fn count_shortest_paths(&self, src: NodeIndex, dst: NodeIndex) -> usize {
        self.paths[src.index()][dst.index()].len()
    }
    pub fn k_shortest_paths(&self, src: NodeIndex, dst: NodeIndex) -> &Vec<Path> {
        &self.paths[src.index()][dst.index()]
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_runs_yens_on_case1() {
        let mut network = Network::new();
        network.add_nodes(4, 0);
        network.add_edges(vec![  // classic trap topology
            (0, 1, 10.0), (1, 2, 10.0), (2, 3, 10.0), (0, 2, 2.0), (1, 3, 1.0)
        ]);

        let mut yens = Yens::new(&network, 10);
        yens.compute_pair(&network, 0.into(), 3.into());

        let kth = |src: usize, dst: usize, kth: usize| {
            yens.kth_shortest_path(src.into(), dst.into(), kth)
                .map(|path| network.node_sequence(&path))
        };

        assert_eq!(yens.count_shortest_paths(0.into(), 3.into()), 4);
        assert_eq!(kth(0, 3, 0), Some(vec![0, 1, 2, 3]));
        assert_eq!(kth(0, 3, 1), Some(vec![0, 2, 3]));
        assert_eq!(kth(0, 3, 2), Some(vec![0, 1, 3]));
        assert_eq!(kth(0, 3, 3), Some(vec![0, 2, 1, 3]));
        assert_eq!(kth(0, 3, 5), None);
    }

    #[test]
    fn it_runs_yens_on_case2() {
        let mut network = Network::new();
        network.add_nodes(6, 93);  // 0..=5 + 99, 6..=98
        network.add_nodes(1, 0);
        network.add_edges(vec![
            (0, 1, 10.0), (1, 2, 20.0), (0, 2, 02.0), (1, 4, 10.0),
            (1, 3, 15.0), (2, 3, 10.0), (2, 4, 10.0)
        ]);
        let more_edges = (4..100).tuple_combinations()
            .map(|(src, dst)| (src, dst, (src * dst) as f64))
            .collect();
        network.add_edges(more_edges);

        let mut yens = Yens::new(&network, 10);
        yens.compute_pair(&network, 0.into(), 2.into());
        yens.compute_pair(&network, 0.into(), 5.into());
        yens.compute_pair(&network, 0.into(), 99.into());

        let kth = |src: usize, dst: usize, kth: usize| {
            yens.kth_shortest_path(src.into(), dst.into(), kth)
                .map(|path| network.node_sequence(&path))
        };

        assert_eq!(yens.count_shortest_paths(0.into(), 2.into()), 4);
        assert_eq!(kth(0, 2, 0), Some(vec![0, 1, 2]));
        assert_eq!(kth(0, 2, 1), Some(vec![0, 1, 3, 2]));
        assert_eq!(kth(0, 2, 2), Some(vec![0, 1, 4, 2]));

        assert_eq!(yens.count_shortest_paths(0.into(), 5.into()), 10);
        assert_eq!(kth(0, 5, 0), Some(vec![0, 1, 4, 99, 5]));
        assert_eq!(kth(0, 5, 1), Some(vec![0, 1, 4, 98, 5]));
        assert_eq!(kth(0, 5, 2), Some(vec![0, 1, 4, 97, 5]));
        assert_eq!(kth(0, 5, 3), Some(vec![0, 1, 4, 99, 98, 5]));
        assert_eq!(kth(0, 5, 4), Some(vec![0, 1, 4, 98, 99, 5]));

        assert_eq!(yens.count_shortest_paths(0.into(), 99.into()), 10);
        assert_eq!(kth(0, 99, 0), Some(vec![0, 1, 4, 99]));
        assert_eq!(kth(0, 99, 1), Some(vec![0, 1, 4, 98, 99]));
        assert_eq!(kth(0, 99, 2), Some(vec![0, 1, 4, 97, 99]));
    }
}
