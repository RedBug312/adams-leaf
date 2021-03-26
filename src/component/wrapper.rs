use std::{ops::Range, rc::Rc};
use crate::{scheduler::schedule_fixed_og, utils::stream::{TSN, AVB}};
use crate::network::MemorizingGraph;
use crate::network::Network;
use crate::component::flowtable::FlowTable;
use crate::component::GCL;
use super::flowtable::FlowArena;

type Route = Vec<usize>;

/// 這個結構預期會被複製很多次，因此其中的每個元件都應儘可能想辦法降低複製成本
#[derive(Clone)]
pub struct NetworkWrapper {
    pub arena: Rc<FlowArena>,
    pub flow_table: FlowTable,
    pub old_new_table: Option<Rc<FlowTable>>, // 在每次運算中類似常數，故用 RC 來包
    pub gcl: GCL,
    pub network: Rc<Network>,
    pub graph: MemorizingGraph,
    pub tsn_fail: bool,
    pub candidates: Vec<Vec<Route>>,
    pub inputs: Range<usize>,
}

impl NetworkWrapper {
    pub fn new(graph: Network) -> Self
    {
        let memorizing = MemorizingGraph::new(&graph);
        NetworkWrapper {
            arena: Rc::new(FlowArena::new()),
            flow_table: FlowTable::new(),
            old_new_table: None,
            gcl: GCL::new(1),
            tsn_fail: false,
            network: Rc::new(graph),
            graph: memorizing,
            candidates: vec![],
            inputs: 0..0,
        }
    }
    /// 插入新的資料流，同時會捨棄先前的新舊表，並創建另一份新舊表
    pub fn insert(&mut self, tsns: Vec<TSN>, avbs: Vec<AVB>) {
        // 釋放舊的表備份表
        // self.old_new_table = None;
        // 插入
        // let new_ids = self.flow_table.insert(tsns, avbs, default_info.clone());
        let arena = Rc::get_mut(&mut self.arena)
            .expect("插入資料流時發生數據爭用");

        let oldlen = arena.len();

        arena.append(tsns, avbs);
        let len = arena.len();

        self.flow_table.resize(len);
        self.old_new_table = Some(Rc::new(self.flow_table.clone()));

        self.inputs = oldlen..len;
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
    pub fn get_old_route(&self, flow_id: usize) -> Option<usize> {
        self.old_new_table
            .as_ref()
            .unwrap()
            .kth_prev(flow_id)
    }
    pub fn adopt_decision(&mut self) {
        self.update_avb();
        self.update_tsn();
        self.flow_table.confirm();
    }
    /// 更新 AVB 資料流表與圖上資訊
    fn update_avb(&mut self) {
        let avbs = &self.arena.avbs;
        let mut updates = Vec::with_capacity(avbs.len());

        updates.extend(self.flow_table.filter_switch(avbs));
        for &id in updates.iter() {
            let prev = self.flow_table.kth_prev(id)
                .expect("Failed to get prev kth with the given id");
            // NOTE: 因為 self.graph 與 self.get_route 是平行所有權
            let graph = unsafe { &mut (*(self as *mut Self)).graph };
            let route = self.get_kth_route(id, prev);
            graph.update_flowid_on_route(false, id, route);
        }

        let avbs = &self.arena.avbs;
        updates.extend(self.flow_table.filter_pending(avbs));
        for &id in updates.iter() {
            let next = self.flow_table.kth_next(id)
                .expect("Failed to get next kth with the given id");
            // NOTE: 因為 self.graph 與 self.get_route 是平行所有權
            let graph = unsafe { &mut (*(self as *mut Self)).graph };
            let route = self.get_kth_route(id, next);
            graph.update_flowid_on_route(true, id, route);
        }
    }
    /// 更新 TSN 資料流表與 GCL
    fn update_tsn(&mut self) {
        let tsns = &self.arena.tsns;
        let mut updates = Vec::with_capacity(tsns.len());

        updates.extend(self.flow_table.filter_switch(tsns));
        for &id in updates.iter() {
            let prev = self.flow_table.kth_prev(id)
                .expect("Failed to get prev kth with the given id");
            let route = self.get_kth_route(id, prev);
            let links = self
                .network
                .get_links_id_bandwidth(route)
                .iter()
                .map(|(ends, _)| *ends)
                .collect();
            self.gcl.delete_flow(&links, id);
        }

        let _self = self as *const Self;
        let arena = Rc::clone(&self.arena);

        let closure = |id| {
            // NOTE: 因為 self.flow_table.get 和 self.get_route_func 和 self.graph 與其它部份是平行所有權
            unsafe {
                let kth = (*_self).flow_table.kth_next(id).unwrap();
                let route = (*_self).candidates[id].get(kth).unwrap();
                (*_self).network.get_links_id_bandwidth(route)
            }
        };

        updates.extend(self.flow_table.filter_pending(tsns));
        // FIXME: stream with choice switch(x, x) is scheduled again
        let result = schedule_fixed_og(&arena, &mut self.gcl, &closure, &updates);
        let result = match result {
            Ok(_) => Ok(false),
            Err(_) => {
                self.gcl.clear();
                schedule_fixed_og(&arena, &mut self.gcl, &closure, &arena.tsns)
                    .and(Ok(true))
            }
        };

        self.tsn_fail = result.is_err();
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

