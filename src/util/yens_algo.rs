extern crate rand;
use rand::{Rng, ThreadRng};

use std::hash::Hash;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::fmt::Debug;

use super::Dijkstra;
use super::MyMinHeap;
use crate::network_struct::OnOffGraph;

type Path<K> = (f64, Vec<K>);

pub struct YensAlgo<'a, K: Hash+Eq+Copy, G: OnOffGraph<K>> {
    g: G,
    k: usize,
    route_table: HashMap<(K, K), Vec<Path<K>>>,
    dijkstra_algo: Dijkstra<'a, K, G>,
    rng: ThreadRng,
}

impl <'a, K: Hash+Eq+Copy+Debug , G: OnOffGraph<K>> YensAlgo<'a, K, G> {
    pub fn new(g: &'a G, k: usize) -> Self {
        return YensAlgo {
            k,
            rng: rand::thread_rng(),
            g: g.clone(),
            route_table: HashMap::new(),
            dijkstra_algo: Dijkstra::new(g)
        }
    }
    pub fn get_routes(&mut self, src: K, dst: K) -> &Vec<Path<K>> {
        let pair = (src, dst);
        if !self.route_table.contains_key(&pair) {
            self.compute_routes(src, dst);
        }
        return self.route_table.get(&pair).unwrap();
    }
    pub fn compute_routes(&mut self, src: K, dst: K) {
        if self.route_table.contains_key(&(src, dst)) {
            return;
        }
        let _self = self as *mut Self;
        let mut paths: HashMap<Rc<Vec<K>>, f64> = HashMap::new();
        let mut visited_edges: HashMap<K, HashSet<K>> = HashMap::new();
        let mut min_heap: MyMinHeap<f64, Rc<Vec<K>>> = MyMinHeap::new();
        let shortest = self.dijkstra_algo.get_route(src, dst);
        min_heap.push(Rc::new(shortest.1), shortest.0, ());
        while let Some((cur_path, dist, _)) = min_heap.pop() {
            paths.insert(cur_path.clone(), dist);
            if paths.len() >= self.k {
                break;
            }
            for i in 0..cur_path.len()-1 {
                let set = visited_edges.entry(cur_path[i]).or_insert(HashSet::new());
                set.insert(cur_path[i+1]);
            }
            self._for_each_deviation(&visited_edges, cur_path.clone(), |next_dist, next_path| {
                let next_path = Rc::new(next_path);
                if !min_heap.contains_key(&next_path) {
                    if !paths.contains_key(&next_path) {
                        // 將給路徑長加上一個隨機的極小值，確保同樣長度的路徑之間存在隨機性
                        let rand_num = unsafe {
                            (*_self).rng.gen_range(1.0, 1.00001)
                        };
                        min_heap.push(next_path.clone(), next_dist * rand_num, ());
                    }
                }
            });
        }
        drop(min_heap);
        let mut vec: Vec<Path<K>> = paths.into_iter().map(|(vec, dist)| {
            if let Ok(vec) = Rc::try_unwrap(vec) {
                (dist, vec)
            } else {
                panic!("取 Rc 值時發生問題");
            }
        }).collect();
        vec.sort_by(|a, b| {
            if a.0 > b.0 {
                return std::cmp::Ordering::Greater;
            } else if a.0 < b.0 {
                return std::cmp::Ordering::Less;
            } else {
                return std::cmp::Ordering::Equal;
            }
        });
        self.route_table.insert((src, dst), vec);
    }
    #[allow(unused_must_use)]
    fn _for_each_deviation(&mut self, visited_edges: &HashMap<K, HashSet<K>>,
        cur_path: Rc<Vec<K>>,
        mut callback: impl FnMut(f64, Vec<K>) -> ()
    ) {
        let mut prefix: Vec<K> = vec![];
        for i in 0..cur_path.len()-1 {
            let cur_node = cur_path[i];
            if i >= 1 {
                self.g.inactivate_node(cur_path[i-1]);
            }
            if let Some(set) = visited_edges.get(&cur_node) {
                for &next_node in set.iter() {
                    self.g.inactivate_edge((cur_node, next_node));
                }
            }
            // ? 這裡是不是有優化的空間?
            let mut spf = Dijkstra::new(&self.g);
            let (_, postfix) = spf.get_route(cur_node, *cur_path.last().unwrap());
            let mut next_path = prefix.clone();
            next_path.extend(postfix);
            callback(self.g.get_dist(&next_path), next_path);
            prefix.push(cur_node);
        }
        self.g.reset();
    }
}

#[cfg(test)]
mod test {
    use crate::network_struct::Graph;
    use crate::algos::StreamAwareGraph;
    use super::YensAlgo;
    #[test]
    fn test_yens_algo1() -> Result<(), String> {
        let mut g = StreamAwareGraph::new();
        g.add_host(Some(100));
        g.add_edge((0, 1), 10.0)?;
        g.add_edge((1, 2), 20.0)?;
        g.add_edge((0, 2), 2.0)?;
        g.add_edge((1, 4), 10.0)?;
        g.add_edge((1, 3), 15.0)?;
        g.add_edge((2, 3), 10.0)?;
        g.add_edge((2, 4), 10.0)?;

        for i in 4..100 {
            for j in i+1..100 {
                g.add_edge((i, j), (i*j) as f64)?;
            }
        }

        let mut algo = YensAlgo::new(&g, 10);
        assert_eq!(vec![0, 1, 2], algo.get_routes(0, 2)[0].1);
        assert_eq!(vec![0, 1, 3, 2], algo.get_routes(0, 2)[1].1);
        assert_eq!(vec![0, 1, 4, 2], algo.get_routes(0, 2)[2].1);
        assert_eq!(4, algo.get_routes(0, 2).len());

        assert_eq!(vec![0, 1, 4, 99], algo.get_routes(0, 99)[0].1);
        assert_eq!(vec![0, 1, 4, 98, 99], algo.get_routes(0, 99)[1].1);
        assert_eq!(vec![0, 1, 4, 97, 99], algo.get_routes(0, 99)[2].1);
        assert_eq!(10, algo.get_routes(0, 99).len());

        assert_eq!(vec![0, 1, 4, 99, 5], algo.get_routes(0, 5)[0].1);
        assert_eq!(vec![0, 1, 4, 98, 5], algo.get_routes(0, 5)[1].1);
        assert_eq!(vec![0, 1, 4, 97, 5], algo.get_routes(0, 5)[2].1);
        assert_eq!(vec![0, 1, 4, 99, 98, 5], algo.get_routes(0, 5)[3].1);
        assert_eq!(vec![0, 1, 4, 98, 99, 5], algo.get_routes(0, 5)[4].1);
        assert_eq!(10, algo.get_routes(0, 5).len());

        return Ok(());
    }
}