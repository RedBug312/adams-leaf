use std::collections::{HashMap, HashSet};
use crate::network::Network;
use crate::component::GateCtrlList;


const KTH_DEFAULT: usize = 0;
type Route = Vec<usize>;


/// 這個結構預期會被複製很多次，因此其中的每個元件都應儘可能想辦法降低複製成本
#[derive(Clone)]
pub struct Decision {
    choices: Vec<Choice>,
    pub candidates: Vec<Vec<Route>>,
    pub allocated_tsns: GateCtrlList,
    pub bypassing_avbs: HashMap<(usize, usize), HashSet<usize>>,
    pub tsn_fail: bool,
}

#[derive(Clone)]
enum Choice {
    Pending(usize),
    Stay(usize),
    Switch(usize, usize),
}


impl Decision {
    pub fn new(graph: &Network) -> Self {
        let bypassing_avbs = graph.edges.keys()
            .map(|&ends| (ends, HashSet::new()))
            .collect();
        Decision {
            choices: vec![],
            candidates: vec![],
            allocated_tsns: GateCtrlList::new(1),
            bypassing_avbs,
            tsn_fail: false,
        }
    }
    pub fn kth(&self, stream: usize) -> Option<usize> {
        self.choices[stream].kth()
    }
    pub fn kth_next(&self, stream: usize) -> Option<usize> {
        self.choices[stream].kth_next()
    }
    pub fn kth_route(&self, stream: usize, kth: usize) -> &Route {
        &self.candidates[stream][kth]
    }
    pub fn route(&self, stream: usize) -> &Route {
        let kth = self.kth(stream).unwrap();
        self.kth_route(stream, kth)
    }
    pub fn route_next(&self, stream: usize) -> &Route {
        let kth_next = self.kth_next(stream).unwrap();
        self.kth_route(stream, kth_next)
    }
    pub fn resize(&mut self, len: usize) {
        let default = Choice::Pending(KTH_DEFAULT);
        self.choices.resize(len, default);
    }
    pub fn pick(&mut self, stream: usize, kth: usize) {
        self.choices[stream].pick(kth);
    }
    pub fn confirm(&mut self) {
        self.choices.iter_mut()
            .for_each(|choice| choice.confirm());
    }
    pub fn filter_pending<'a>(&'a self, source: &'a Vec<usize>)
        -> impl Iterator<Item=usize> + 'a {
        source.iter().cloned()
            .filter(move |&id| matches!(self.choices[id],
                    Choice::Pending(_)))
    }
    pub fn filter_switch<'a>(&'a self, source: &'a Vec<usize>)
        -> impl Iterator<Item=usize> + 'a {
        source.iter().cloned()
            .filter(move |&id| matches!(self.choices[id],
                    Choice::Switch(prev, next) if prev != next))
    }
}

impl Decision {
    /// 確定一條資料流的路徑時，將該資料流的ID記憶在它經過的邊上，移除路徑時則將ID遺忘。
    ///
    /// __注意：此處兩個方向不視為同個邊！__
    /// * `remember` - 布林值，記憶或是遺忘路徑
    /// * `stream` - 要記憶或遺忘的資料流ID
    /// * `route` - 該路徑(以節點組成)
    pub fn insert_bypassing_avb_on_kth_route(&mut self, stream: usize, kth: usize) {
        let route = &self.candidates[stream][kth];  // kth_route without clone
        for ends in route.windows(2) {
            let ends = (ends[0], ends[1]);
            let set = self.bypassing_avbs.get_mut(&ends)
                .expect("Failed to insert bypassing avb into an invalid edge");
            set.insert(stream);
        }
    }
    pub fn remove_bypassing_avb_on_kth_route(&mut self, stream: usize, kth: usize) {
        let route = &self.candidates[stream][kth];  // kth_route without clone
        for ends in route.windows(2) {
            let ends = (ends[0], ends[1]);
            let set = self.bypassing_avbs.get_mut(&ends)
                .expect("Failed to remove bypassing avb from an invalid edge");
            set.remove(&stream);
        }
    }
}

impl Choice {
    fn kth(&self) -> Option<usize> {
        match self {
            Choice::Pending(_)      => None,
            Choice::Stay(prev)      => Some(*prev),
            Choice::Switch(prev, _) => Some(*prev),
        }
    }
    fn kth_next(&self) -> Option<usize> {
        match self {
            Choice::Pending(next)   => Some(*next),
            Choice::Stay(prev)      => Some(*prev),
            Choice::Switch(_, next) => Some(*next),
        }
    }
    fn pick(&mut self, next: usize) {
        *self = match self {
            Choice::Pending(_)      => Choice::Pending(next),
            Choice::Stay(prev)      => Choice::Switch(*prev, next),
            Choice::Switch(prev, _) => Choice::Switch(*prev, next),
        };
    }
    fn confirm(&mut self) {
        *self = match self {
            Choice::Pending(next)   => Choice::Stay(*next),
            Choice::Stay(prev)      => Choice::Stay(*prev),
            Choice::Switch(_, next) => Choice::Stay(*next),
        };
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::flow::{data::TSNData, TSNFlow};
    use crate::read_topo_from_file;
    use std::collections::HashMap;

    struct Env(HashMap<(usize, usize), Vec<Route>>);
    impl Env {
        pub fn new() -> Self {
            let mut map = HashMap::new();
            map.insert((0, 4), vec![vec![0, 4], vec![0, 5, 4]]);
            map.insert((1, 2), vec![vec![1, 0, 4, 2]]);
            Env(map)
        }
        pub fn get_route(&self, src: usize, dst: usize, i: usize) -> *const Route {
            &self.0.get(&(src, dst)).unwrap()[i]
        }
    }

    fn init() -> (NetworkWrapper<usize>, Vec<TSNFlow>) {
        let graph = read_topo_from_file("test_graph.json");
        let env = Env::new();
        let wrapper = NetworkWrapper::new(graph, move |flow, k: &usize| match flow {
            FlowEnum::AVB(flow) => env.get_route(flow.src, flow.dst, *k),
            FlowEnum::TSN(flow) => env.get_route(flow.src, flow.dst, *k),
        });
        let flows = vec![
            TSNFlow {
                id: 0.into(),
                src: 0,
                dst: 4,
                size: 100,
                period: 100,
                max_delay: 100,
                spec_data: TSNData { offset: 0 },
            },
            TSNFlow {
                id: 0.into(),
                src: 0,
                dst: 4,
                size: 100,
                period: 150,
                max_delay: 150,
                spec_data: TSNData { offset: 0 },
            },
            TSNFlow {
                id: 0.into(),
                src: 1,
                dst: 2,
                size: 100,
                period: 200,
                max_delay: 200,
                spec_data: TSNData { offset: 0 },
            },
        ];
        (wrapper, flows)
    }

    #[test]
    fn test_insert_get_route() {
        let (mut wrapper, flows) = init();
        wrapper.insert(flows.clone(), vec![], 0);

        wrapper.flow_table.update_info(1.into(), 1);

        assert_eq!(&vec![0, 4], wrapper.get_route(0.into()));
        assert_eq!(&vec![0, 5, 4], wrapper.get_route(1.into()));
        assert_eq!(&vec![1, 0, 4, 2], wrapper.get_route(2.into()));
        let old_new = wrapper
            .old_new_table
            .as_ref()
            .unwrap()
            .get_info(1.into())
            .unwrap();
        assert_eq!(&OldNew::New, old_new);

        wrapper.insert(flows.clone(), vec![], 0);
        assert_eq!(&vec![0, 4], wrapper.get_route(3.into()));
        assert_eq!(&vec![0, 4], wrapper.get_route(4.into()));
        assert_eq!(&vec![1, 0, 4, 2], wrapper.get_route(5.into()));
        let old_new = wrapper
            .old_new_table
            .as_ref()
            .unwrap()
            .get_info(1.into())
            .unwrap();
        assert_eq!(&OldNew::Old(1), old_new);
        let old_new = wrapper
            .old_new_table
            .as_ref()
            .unwrap()
            .get_info(3.into())
            .unwrap();
        assert_eq!(&OldNew::New, old_new);
    }
    #[test]
    #[should_panic]
    fn test_clone_and_insert_should_panic() {
        let (mut wrapper, flows) = init();
        wrapper.insert(flows.clone(), vec![], 0);
        let mut wrapper2 = wrapper.clone();
        wrapper2.insert(flows.clone(), vec![], 0);
    }
    #[test]
    fn test_clone() {
        let (mut wrapper, flows) = init();
        wrapper.insert(flows.clone(), vec![], 0);
        let wrapper2 = wrapper.clone();
        wrapper.flow_table.update_info(0.into(), 99);
        assert_eq!(&99, wrapper.flow_table.get_info(0.into()).unwrap());
        assert_eq!(&0, wrapper2.flow_table.get_info(0.into()).unwrap());
    }
    fn build_id_vec(v: Vec<usize>) -> Vec<usize> {
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

        let mut ans: Vec<Vec<usize>> = vec![vec![], vec![], vec![]];
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

