use std::{ops::Range, rc::Rc};
use crate::{scheduler::schedule_fixed_og, utils::stream::{TSN, AVB}};
use crate::network::MemorizingGraph;
use crate::network::Network;
use crate::component::flowtable::FlowTable;
use crate::component::GCL;
use super::{cost::{RoutingCost, Calculator}, flowtable::FlowArena};

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

        let (mut new_tsns, mut new_avbs) = arena.append(tsns, avbs);
        let len = arena.len();

        let mut old_new_table = self.flow_table.clone();
        old_new_table.resize_pending(len);
        self.old_new_table = Some(Rc::new(old_new_table));

        self.flow_table.resize_switch(len);

        self.flow_table.tsn_diff.append(&mut new_tsns);
        self.flow_table.avb_diff.append(&mut new_avbs);

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
    pub fn update_single_avb(&mut self, id: usize, info: usize) {
        // NOTE: 因為 self.graph 與 self.get_route 是平行所有權
        let graph = unsafe { &mut (*(self as *mut Self)).graph };
        let og_route = self.get_route(id);
        // 忘掉舊的
        graph.update_flowid_on_route(false, id, og_route);
        self.flow_table.update_info(id, info);
        let new_route = self.get_route(id);
        // 記憶新的
        graph.update_flowid_on_route(true, id, new_route);
    }
    /// 更新 AVB 資料流表與圖上資訊
    pub fn update_avb(&mut self) {
        let avb_diff = self.flow_table.avb_diff.clone();
        for &id in avb_diff.iter() {
            let prev = self.flow_table.kth_prev(id);
            let next = self.flow_table.kth_next(id)
                .expect("Failed to get next kth with the given id");
            // NOTE: 因為 self.graph 與 self.get_route 是平行所有權
            let graph = unsafe { &mut (*(self as *mut Self)).graph };
            // 忘掉舊的
            if prev.is_some() {
                let route = self.get_kth_route(id, prev.unwrap());
                graph.update_flowid_on_route(false, id, route);
            }
            // 記憶新的
            let route = self.get_kth_route(id, next);
            graph.update_flowid_on_route(true, id, route);
            // self.flow_table.update_info(id, next);
        }
        self.flow_table.apply(false);
        self.flow_table.avb_diff = vec![];

    }
    /// 更新 TSN 資料流表與 GCL
    pub fn update_tsn(&mut self) {
        // NOTE: 在 schedule_online 函式中就會更新資料流表（這當然是個不太好的實作……）
        //       因此在這裡就不用執行 self.flow_table.update_info()

        for &id in self.flow_table.iter_tsn_diff() {
            // NOTE: 拔除 GCL
            let prev = self.flow_table.kth_prev(id);
            if prev.is_none() { continue; }
            let route = self.get_kth_route(id, prev.unwrap());
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

        // FIXME: stream with choice switch(x, x) is scheduled again
        let result = schedule_fixed_og(&arena, &mut self.gcl, &closure, &self.flow_table.tsn_diff);
        let result = match result {
            Ok(_) => Ok(false),
            Err(_) => {
                self.gcl.clear();
                schedule_fixed_og(&arena, &mut self.gcl, &closure, &arena.tsns)
                    .and(Ok(true))
            }
        };

        // let result = schedule_online(&arena, &mut self.flow_table, diff, &mut self.gcl, &closure);

        if result.is_err() {
            self.tsn_fail = true;
        } else {
            // TODO: 應該如何處理 result = Ok(bool) ？
            self.tsn_fail = false;
        }
        // self.flow_table.apply_diff(true, diff);
        self.flow_table.apply(true);
        self.flow_table.tsn_diff = vec![];
    }
    pub fn get_flow_table(&self) -> &FlowTable {
        &self.flow_table
    }
    /// 路徑為可選參數，若不給代表照資料流表來走
    pub fn compute_avb_wcd(&self, flow: usize, route: Option<usize>) -> u32 {
        self._compute_avb_wcd(flow, route)
    }
    pub fn compute_all_cost(&self) -> RoutingCost {
        self._compute_all_cost()
    }
    pub fn compute_single_avb_cost(&self, flow: usize) -> RoutingCost {
        self._compute_single_avb_cost(flow)
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

