use super::Network;
use crate::utils::stream::FlowID;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// 每條邊上記憶了其承載的資料流識別碼。使用淺層複製，圖的節點、邊、頻寬、開關等資訊都將共用，僅有記憶被複製。
#[derive(Clone)]
pub struct MemorizingGraph {
    inner: Rc<Network>,
    edge_info: HashMap<(usize, usize), HashSet<FlowID>>,
}

impl std::ops::Deref for MemorizingGraph {
    type Target = Network;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl MemorizingGraph {
    pub fn new(graph: Network) -> Self {
        let mut edge_info = HashMap::<(usize, usize), HashSet<FlowID>>::new();
        for (key, _) in graph.edges.iter() {
            edge_info.insert(key.clone(), HashSet::new());
        }
        MemorizingGraph {
            inner: Rc::new(graph),
            edge_info,
        }
    }
    /// 確定一條資料流的路徑時，將該資料流的ID記憶在它經過的邊上，移除路徑時則將ID遺忘。
    ///
    /// __注意：此處兩個方向不視為同個邊！__
    /// * `remember` - 布林值，記憶或是遺忘路徑
    /// * `flow_id` - 要記憶或遺忘的資料流ID
    /// * `route` - 該路徑(以節點組成)
    pub fn update_flowid_on_route(&mut self, remember: bool, flow_id: FlowID, route: &Vec<usize>) {
        for i in 0..route.len() - 1 {
            let set = self.edge_info.get_mut(&(route[i], route[i + 1])).unwrap();
            if remember {
                set.insert(flow_id);
            } else {
                set.remove(&flow_id);
            }
        }
    }
    /// 把邊上記憶的資訊通通忘掉！
    pub fn forget_all_flows(&mut self) {
        for (_, set) in self.edge_info.iter_mut() {
            *set = HashSet::new();
        }
    }
    /// 詢問一條路徑上所有共用過邊的資料流。針對路上每個邊都會回傳一個陣列，內含走了這個邊的資料流（空陣列代表無人走過）
    ///
    /// __注意：方向不同者不視為共用！__
    pub fn get_overlap_flows(&self, route: &Vec<usize>) -> Vec<Vec<FlowID>> {
        // TODO 回傳的 Vec<Vec> 有優化空間
        let mut ret = Vec::with_capacity(route.len() - 1);
        for i in 0..route.len() - 1 {
            if let Some(set) = self.edge_info.get(&(route[i], route[i + 1])) {
                ret.push(set.iter().map(|id| *id).collect());
            } else {
                panic!("{} {} 之間沒有連線", route[i], route[i + 1]);
            }
        }
        ret
    }
}

#[cfg(test)]
mod test {
    use super::super::*;
    use super::*;
    fn build_id_vec(v: Vec<usize>) -> Vec<FlowID> {
        v.into_iter().map(|i| i.into()).collect()
    }
    #[test]
    fn test_remember_forget_flow() -> Result<(), String> {
        let mut g = Network::new();
        g.add_host(Some(5));
        g.add_edge((0, 1), 10.0)?;
        g.add_edge((1, 2), 20.0)?;
        g.add_edge((2, 3), 2.0)?;
        g.add_edge((0, 3), 2.0)?;
        g.add_edge((0, 4), 2.0)?;
        g.add_edge((3, 4), 2.0)?;

        let mut g = MemorizingGraph::new(g);

        let mut ans: Vec<Vec<FlowID>> = vec![vec![], vec![], vec![]];
        assert_eq!(ans, g.get_overlap_flows(&vec![0, 3, 2, 1]));

        g.update_flowid_on_route(true, 0.into(), &vec![2, 3, 4]);
        g.update_flowid_on_route(true, 1.into(), &vec![1, 0, 3, 4]);

        assert_eq!(ans, g.get_overlap_flows(&vec![4, 3, 0, 1])); // 兩個方向不視為重疊

        let mut ov_flows = g.get_overlap_flows(&vec![0, 3, 4]);
        assert_eq!(build_id_vec(vec![1]), ov_flows[0]);
        ov_flows[1].sort();
        assert_eq!(build_id_vec(vec![0, 1]), ov_flows[1]);

        g.update_flowid_on_route(false, 1.into(), &vec![1, 0, 3, 4]);
        ans = vec![vec![], vec![0.into()]];
        assert_eq!(ans, g.get_overlap_flows(&vec![0, 3, 4]));

        g.forget_all_flows();
        ans = vec![vec![], vec![]];
        assert_eq!(ans, g.get_overlap_flows(&vec![0, 3, 4]));

        Ok(())
    }
}
