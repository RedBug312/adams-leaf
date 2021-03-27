use crate::network::MemorizingGraph;
use crate::network::Network;
use crate::component::flowtable::FlowTable;
use crate::component::GCL;


type Route = Vec<usize>;


/// 這個結構預期會被複製很多次，因此其中的每個元件都應儘可能想辦法降低複製成本
#[derive(Clone)]
pub struct NetworkWrapper {
    pub flow_table: FlowTable,
    pub gcl: GCL,
    pub graph: MemorizingGraph,
    pub candidates: Vec<Vec<Route>>,
    pub tsn_fail: bool,
}

impl NetworkWrapper {
    pub fn new(graph: &Network) -> Self
    {
        let memorizing = MemorizingGraph::new(graph);
        NetworkWrapper {
            flow_table: FlowTable::new(),
            gcl: GCL::new(1),
            graph: memorizing,
            candidates: vec![],
            tsn_fail: false,
        }
    }
    /// 插入新的資料流，同時會捨棄先前的新舊表，並創建另一份新舊表
    pub fn resize(&mut self, len: usize) {
        self.flow_table.resize(len);
    }
    pub fn get_route(&self, flow_id: usize) -> &Route {
        let kth = self.flow_table.kth_next(flow_id).unwrap();
        let route = self.candidates[flow_id].get(kth).unwrap();
        route
    }
    pub fn get_kth_route(&self, flow_id: usize, kth: usize) -> &Route {
        let route = self.candidates[flow_id].get(kth).unwrap();
        route
    }
    pub fn get_flow_table(&self) -> &FlowTable {
        &self.flow_table
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
}

