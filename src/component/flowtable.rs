use crate::utils::stream::{AVBFlow, FlowEnum, FlowID, TSNFlow};
use std::rc::Rc;

#[derive(Clone)]
enum Action {
    Pending,
    Init(usize),
    Keep(usize),
    Move(usize),
}

pub struct FlowArena {
    avbs: Vec<FlowID>,
    tsns: Vec<FlowID>,
    flow_list: Vec<FlowEnum>,
    max_id: FlowID,
}
impl FlowArena {
    fn new() -> Self {
        FlowArena {
            avbs: vec![],
            tsns: vec![],
            flow_list: vec![],
            max_id: 0.into(),
        }
    }
    fn insert<T: Into<FlowEnum>>(&mut self, flow: T) -> FlowID {
        let id = FlowID(self.flow_list.len());
        let mut flow: FlowEnum = flow.into();
        match &mut flow {
            FlowEnum::AVB(ref mut inner) => {
                inner.id = id;
                self.avbs.push(id);
            }
            FlowEnum::TSN(ref mut inner) => {
                inner.id = id;
                self.tsns.push(id)
            }
        }
        self.flow_list.push(flow);
        self.max_id = std::cmp::max(self.max_id, id);
        id
    }
    fn get(&self, id: FlowID) -> Option<&FlowEnum> {
        if id.0 < self.flow_list.len() {
            return self.flow_list.get(id.0);
        }
        return None;
    }
}

pub trait IFlowTable {
    fn get_inner_arena(&self) -> &Rc<FlowArena>;
    fn get_info(&self, id: FlowID) -> Option<usize>;
    fn update_info(&mut self, id: FlowID, info: usize);
    fn check_exist(&self, id: FlowID) -> bool {
        self.get_info(id).is_some()
    }
    fn get(&self, id: FlowID) -> Option<&FlowEnum> {
        self.get_inner_arena().get(id)
    }
    fn get_avb(&self, id: FlowID) -> Option<&AVBFlow> {
        if self.check_exist(id) {
            if let Some(FlowEnum::AVB(flow)) = self.get_inner_arena().get(id) {
                return Some(flow);
            }
        }
        None
    }
    fn get_tsn(&self, id: FlowID) -> Option<&TSNFlow> {
        if self.check_exist(id) {
            if let Some(FlowEnum::TSN(flow)) = self.get_inner_arena().get(id) {
                return Some(flow);
            }
        }
        None
    }
    fn is_same_flow_list<T: IFlowTable>(&self, other: &T) -> bool {
        let a = &**self.get_inner_arena() as *const FlowArena;
        let b = &**other.get_inner_arena() as *const FlowArena;
        a == b
    }
    /// 建立一個新的資料流表。邏輯上，這個新資料流表為空，但可以執行 update_info。
    /// 遍歷新產生的表時，會自動跳過沒有修改過的資料流，且效能較高。
    /// # 範例
    /// ```
    /// let mut table = FlowTable::<usize>::new();
    /// table.insert(vec![flow0, flow1], 0);
    /// // table 中有兩個資料流，隨附資訊皆為0
    /// let mut changed_table = table.clone_as_diff();
    /// // changed_table 中有零個資料流
    /// changed_table.update(1, 99);
    /// // changed_table 中有一個 id=1 的資料流，且隨附資訊為99
    /// changed_table.insert(vec![flow2], 0);
    /// // will panic!
    /// ```
    fn clone_as_diff(&self) -> DiffFlowTable;
    fn get_avb_cnt(&self) -> usize;
    fn get_tsn_cnt(&self) -> usize;
    fn get_flow_cnt(&self) -> usize {
        self.get_tsn_cnt() + self.get_avb_cnt()
    }
    fn get_max_id(&self) -> FlowID {
        self.get_inner_arena().max_id
    }
    fn iter_tsn<'a>(&'a self) -> Box<dyn Iterator<Item=&TSNFlow> + 'a>;
}

/// 儲存的資料分為兩部份：資料流本身，以及隨附的資訊（T）。
///
/// __注意！這個資料結構 clone 的時候並不會把所有資料流複製一次，只會複製資訊的部份。__
///
/// 此處隱含的假設為：資料流本身不會時常變化，在演算法執行的過程中應該是唯一不變的，因此用一個 Rc 來記憶即可。
///
/// TODO 觀察在大資料量下這個改動是否有優化的效果。在小資料量下似乎沒啥差別。
#[derive(Clone)]
pub struct FlowTable {
    arena: Rc<FlowArena>,
    infos: Vec<Action>,
    avb_cnt: usize,
    tsn_cnt: usize,
}
impl FlowTable {
    pub fn new() -> Self {
        FlowTable {
            infos: vec![],
            arena: Rc::new(FlowArena::new()),
            avb_cnt: 0,
            tsn_cnt: 0,
        }
    }
    pub fn apply_diff(&mut self, is_tsn: bool, other: &DiffFlowTable) {
        if !self.is_same_flow_list(other) {
            panic!("試圖合併不相干的資料流表");
        }
        if is_tsn {
            for flow in other.iter_tsn() {
                let info = other.get_info(flow.id).unwrap();
                self.update_info(flow.id, info.clone());
            }
        } else {
            for flow in other.iter_avb() {
                let info = other.get_info(flow.id).unwrap();
                self.update_info(flow.id, info.clone());
            }
        }
    }
    pub fn insert_xxx(&mut self, flows: Vec<FlowID>) {
        for id in flows {
            let id: usize = id.into();
            self.infos[id] = Action::Pending;
        }
    }
    pub fn insert(
        &mut self,
        tsns: Vec<TSNFlow>,
        avbs: Vec<AVBFlow>,
        default_info: usize,
    ) -> Vec<FlowID> {
        let arena = Rc::get_mut(&mut self.arena).expect("插入資料流時發生數據爭用");
        let mut id_list = vec![];
        for flow in tsns.into_iter() {
            let id = arena.insert(flow);
            self.infos.push(Action::Init(default_info.clone()));
            id_list.push(id);
            self.tsn_cnt += 1;
        }
        for flow in avbs.into_iter() {
            let id = arena.insert(flow);
            self.infos.push(Action::Init(default_info.clone()));
            id_list.push(id);
            self.avb_cnt += 1;
        }
        id_list
    }
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=&FlowEnum> + 'a {
        self.arena.flow_list.iter()
    }
    pub fn iter_avb<'a>(&'a self) -> impl Iterator<Item=&AVBFlow> + 'a {
        let iterator = self.arena.avbs.iter()
            .filter_map(move |id| self.arena.flow_list.get(id.0))
            .map(|flow| flow.avb());
        iterator
    }
}
impl IFlowTable for FlowTable {
    fn get_avb_cnt(&self) -> usize {
        self.avb_cnt
    }
    fn get_tsn_cnt(&self) -> usize {
        self.tsn_cnt
    }
    fn get_inner_arena(&self) -> &Rc<FlowArena> {
        &self.arena
    }
    fn get_info(&self, id: FlowID) -> Option<usize> {
        match self.infos.get(id.0) {
            Some(Action::Pending) => None,
            Some(Action::Init(info)) => Some(*info),
            Some(Action::Keep(info)) => Some(*info),
            Some(Action::Move(info)) => Some(*info),
            None => panic!("Failed to get info from an invalid id"),
        }
    }
    fn update_info(&mut self, id: FlowID, info: usize) {
        debug_assert!(id.0 < self.infos.len());
        self.infos[id.0] = Action::Init(info);
    }

    fn clone_as_diff(&self) -> DiffFlowTable {
        DiffFlowTable::new(self)
    }
    fn iter_tsn<'a>(&'a self) -> Box<dyn Iterator<Item=&TSNFlow> + 'a> {
        let iterator = self.arena.tsns.iter()
            .filter_map(move |id| self.arena.flow_list.get(id.0))
            .map(|flow| flow.tsn());
        Box::new(iterator)
    }
}

#[derive(Clone)]
pub struct DiffFlowTable {
    avb_diff: Vec<FlowID>,
    tsn_diff: Vec<FlowID>,
    arena: Rc<FlowArena>,
    infos: Vec<Action>,
    avb_cnt: usize,
    tsn_cnt: usize,
}
impl DiffFlowTable {
    pub fn new(og_table: &FlowTable) -> Self {
        DiffFlowTable {
            avb_diff: vec![],
            tsn_diff: vec![],
            arena: og_table.arena.clone(),
            infos: og_table
                .infos
                .iter()
                .map(|action| match action {
                    Action::Pending => Action::Pending,
                    Action::Init(t) => Action::Keep(*t),
                    Action::Keep(t) => Action::Keep(*t),
                    Action::Move(t) => Action::Keep(*t),
                })
                .collect(),
            avb_cnt: 0,
            tsn_cnt: 0,
        }
    }
    /// 不管是否和本來相同，硬是更新
    pub fn update_info_force(&mut self, id: FlowID, info: usize) {
        if !self.check_exist(id) {
            match self.get_xxx(id).unwrap() {
                FlowEnum::TSN(_) => self.tsn_diff.push(id),
                FlowEnum::AVB(_) => self.avb_diff.push(id),
            }
        }
        self.update_info_xxx(id, Action::Move(info));
    }
    pub fn iter_avb<'a>(&'a self) -> Box<dyn Iterator<Item=&AVBFlow> + 'a> {
        let iterator = self.avb_diff.iter()
            .filter_map(move |id| self.get_inner_arena().flow_list.get(id.0))
            .map(|flow| flow.avb());
        Box::new(iterator)
    }
    fn get_xxx(&self, id: FlowID) -> Option<&FlowEnum> {
        self.arena.get(id)
    }
    fn get_info_xxx(&self, id: FlowID) -> Option<&Action> {
        debug_assert!(id.0 < self.infos.len());
        self.infos.get(id.0)
    }
    fn update_info_xxx(&mut self, id: FlowID, info: Action) {
        debug_assert!(id.0 < self.infos.len());
        self.infos[id.0] = info;
    }
}
impl IFlowTable for DiffFlowTable {
    fn get_avb_cnt(&self) -> usize {
        self.avb_diff.len()
    }
    fn get_tsn_cnt(&self) -> usize {
        self.tsn_diff.len()
    }
    fn get_inner_arena(&self) -> &Rc<FlowArena> {
        &self.arena
    }
    fn get_info(&self, id: FlowID) -> Option<usize> {
        if let Some(Action::Move(info)) = self.get_info_xxx(id) {
            Some(*info)
        } else {
            None
        }
    }
    fn update_info(&mut self, id: FlowID, info: usize) {
        if let Some(Action::Keep(og_value)) = self.get_info_xxx(id) {
            // NOTE: 若和本來值相同，就啥都不做
            if *og_value == info {
                return;
            }
            match self.get_xxx(id).unwrap() {
                FlowEnum::TSN(_) => self.tsn_diff.push(id),
                FlowEnum::AVB(_) => self.avb_diff.push(id),
            }
        }
        // NOTE: 如果本來就是 New，就不推進 diff 表（因為之前推過了）
        self.update_info_xxx(id, Action::Move(info));
    }
    fn clone_as_diff(&self) -> DiffFlowTable {
        self.clone()
    }
    fn iter_tsn<'a>(&'a self) -> Box<dyn Iterator<Item=&TSNFlow> + 'a> {
        let iterator = self.tsn_diff.iter()
            .filter_map(move |id| self.get_inner_arena().flow_list.get(id.0))
            .map(|flow| flow.tsn());
        Box::new(iterator)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::read_flows_from_file;
    #[test]
    #[should_panic]
    fn datarace_should_panic() {
        let mut table = FlowTable::<usize>::new();
        let _table2 = table.clone();
        // drop(_table2);
        table.insert(vec![], vec![], 0);
    }
    #[test]
    fn no_datarace_no_panic() {
        let mut table = FlowTable::<usize>::new();
        let _table2 = table.clone();
        drop(_table2);
        table.insert(vec![], vec![], 0);
    }
    #[test]
    fn test_diff_flow_table() {
        let mut table = FlowTable::<usize>::new();
        let (tsns, avbs) = read_flows_from_file("test_flow.json", 1);
        assert_eq!(1, tsns.len());
        assert_eq!(5, avbs.len());
        assert_eq!(FlowID(0), table.get_max_id());
        table.insert(tsns, avbs, 0);
        assert_eq!(FlowID(5), table.get_max_id());
        assert_eq!(count_flows_iterative(&table), 6);
        assert_eq!(table.get_flow_cnt(), 6);

        assert_eq!(1, table.get_tsn_cnt());
        assert_eq!(5, table.get_avb_cnt());

        let mut changed = table.clone_as_diff();
        assert_eq!(changed.get_flow_cnt(), 0);
        assert_eq!(count_flows_iterative(&changed), 0);

        changed.update_info(2.into(), 99);
        assert_eq!(changed.get_flow_cnt(), 1);
        assert_eq!(count_flows_iterative(&changed), 1);

        changed.update_info(4.into(), 77);
        assert_eq!(changed.get_flow_cnt(), 2);
        assert_eq!(count_flows_iterative(&changed), 2);

        assert_eq!(changed.get_info(0.into()), None);
        assert_eq!(changed.get_info(2.into()), Some(&99));
        assert_eq!(changed.get_info(4.into()), Some(&77));
        assert_eq!(table.get_info(0.into()), Some(&0));
        assert_eq!(table.get_info(2.into()), Some(&0));
        assert_eq!(table.get_info(4.into()), Some(&0));

        // 改動一筆 TSN 資料流的隨附資訊
        changed.update_info(0.into(), 66);
        assert_eq!(changed.get_flow_cnt(), 3);
        assert_eq!(changed.get_info(0.into()), Some(&66));

        // 由於只合併 AVB 的部份，識別碼=0的資料流應不受影響
        table.apply_diff(false, &changed);
        assert_eq!(table.get_info(0.into()), Some(&0));
        assert_eq!(table.get_info(2.into()), Some(&99));
        assert_eq!(table.get_info(4.into()), Some(&77));
        assert_eq!(table.get_flow_cnt(), 6);
        assert_eq!(count_flows_iterative(&table), 6);
    }
    #[test]
    fn test_insert_return_id() {
        let mut table = FlowTable::<usize>::new();
        let (tsns, avbs) = read_flows_from_file("test_flow.json", 1);
        let new_ids = table.insert(tsns, avbs, 0);
        assert_eq!(6, new_ids.len());
        assert_eq!(FlowID(0), new_ids[0]);
        assert_eq!(FlowID(1), new_ids[1]);
        assert_eq!(FlowID(2), new_ids[2]);
        assert_eq!(FlowID(3), new_ids[3]);
        assert_eq!(FlowID(5), new_ids[5]);
    }
    #[test]
    #[should_panic]
    fn apply_diff_different_flows_should_panic() {
        let mut table = FlowTable::<usize>::new();
        let (tsns, avbs) = read_flows_from_file("test_flow.json", 1);
        table.insert(tsns.clone(), avbs.clone(), 0);
        let table2 = FlowTable::<usize>::new().clone_as_diff();
        table.insert(tsns, avbs, 0);
        table.apply_diff(true, &table2);
    }
    #[test]
    fn test_flowtable_iterator() {
        let mut table = FlowTable::<usize>::new();
        let (tsns, avbs) = read_flows_from_file("test_flow.json", 1);
        table.insert(tsns, avbs, 99);

        let mut first = true;
        for (flow, &data) in table.iter_tsn() {
            assert_eq!(FlowID(0), flow.id);
            assert_eq!(99, data);
            assert!(first); // 只會來一次
            first = false;
        }
        assert!(!first);

        for (flow, data) in table.iter_avb_mut() {
            assert_eq!(data, &99);
            *data = flow.id.into()
        }

        for (flow, &data) in table.iter_avb() {
            assert_eq!(flow.id, FlowID(data));
        }
    }
    #[test]
    fn test_difftable_iterator() {
        let mut table = FlowTable::<usize>::new();
        let (tsns, avbs) = read_flows_from_file("test_flow.json", 1);
        table.insert(tsns, avbs, 99);
        let mut change = table.clone_as_diff();
        for _ in change.iter_avb() {
            panic!("不該走進來！");
        }
        for _ in change.iter_tsn() {
            panic!("不該走進來！");
        }
        change.update_info(0.into(), 77);

        let mut first = true;
        for (flow, &data) in table.iter_tsn() {
            assert_eq!(FlowID(0), flow.id);
            assert_eq!(99, data);
            assert!(first); // 只會來一次
            first = false;
        }
        assert!(!first);

        let mut first = true;
        for (flow, data) in change.iter_tsn_mut() {
            assert_eq!(FlowID(0), flow.id);
            assert_eq!(77, *data);
            assert!(first); // 只會來一次
            *data = 9;
            first = false;
        }
        assert!(!first);
        assert_eq!(&9, change.get_info(0.into()).unwrap());

        change.update_info(3.into(), 55);

        let mut first = true;
        for (flow, &data) in change.iter_avb() {
            assert_eq!(FlowID(3), flow.id);
            assert_eq!(55, data);
            assert!(first); // 只會來一次
            first = false;
        }
        assert!(!first);
    }
    #[test]
    fn test_clone_as_type() {
        let mut table = FlowTable::<usize>::new();
        let (tsns, avbs) = read_flows_from_file("test_flow.json", 1);
        table.insert(tsns, avbs, 99);
        table.update_info(2.into(), 77);

        let new_table = table.clone_as_type(|id, t| {
            if table.get_tsn(id).is_some() {
                format!("tsn, id={}, og_value={}", id.0, t)
            } else {
                format!("avb, id={}, og_value={}", id.0, t)
            }
        });

        assert_eq!(
            Some(&"tsn, id=0, og_value=99".to_owned()),
            new_table.get_info(0.into())
        );
        assert_eq!(
            Some(&"avb, id=1, og_value=99".to_owned()),
            new_table.get_info(1.into())
        );
        assert_eq!(
            Some(&"avb, id=2, og_value=77".to_owned()),
            new_table.get_info(2.into())
        );
        assert_eq!(
            Some(&"avb, id=3, og_value=99".to_owned()),
            new_table.get_info(3.into())
        );
        assert_eq!(None, new_table.get_info(8.into()));
    }
    fn count_flows_iterative<FT: IFlowTable<INFO = usize>>(table: &FT) -> usize {
        let mut cnt = 0;
        for _ in table.iter_avb() {
            cnt += 1;
        }
        for _ in table.iter_tsn() {
            cnt += 1;
        }
        let mut cnt2 = 0;
        for _ in table.iter() {
            cnt2 += 1;
        }
        assert_eq!(cnt, cnt2);
        cnt
    }
}
