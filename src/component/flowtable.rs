use iter::{Iter, IterMut};
use crate::utils::stream::{AVBFlow, FlowEnum, FlowID, TSNFlow};
use std::rc::Rc;
pub struct FlowArena {
    avbs: Vec<Option<FlowID>>,
    tsns: Vec<Option<FlowID>>,
    flow_list: Vec<Option<FlowEnum>>,
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
                self.avbs.push(Some(id));
            }
            FlowEnum::TSN(ref mut inner) => {
                inner.id = id;
                self.tsns.push(Some(id))
            }
        }
        self.flow_list.push(Some(flow));
        self.max_id = std::cmp::max(self.max_id, id);
        id
    }
    fn get(&self, id: FlowID) -> Option<&FlowEnum> {
        if id.0 < self.flow_list.len() {
            return self.flow_list[id.0].as_ref();
        }
        return None;
    }
}

pub trait IFlowTable {
    type INFO: Clone + Eq;
    fn get_inner_arena(&self) -> &Rc<FlowArena>;
    fn get_info(&self, id: FlowID) -> Option<&Self::INFO>;
    fn update_info(&mut self, id: FlowID, info: Self::INFO);
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
    fn is_same_flow_list<T: IFlowTable<INFO = Self::INFO>>(&self, other: &T) -> bool {
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
    fn clone_as_diff(&self) -> DiffFlowTable<Self::INFO>;
    fn get_avb_cnt(&self) -> usize;
    fn get_tsn_cnt(&self) -> usize;
    fn get_flow_cnt(&self) -> usize {
        self.get_tsn_cnt() + self.get_avb_cnt()
    }
    fn get_max_id(&self) -> FlowID {
        self.get_inner_arena().max_id
    }
    fn iter_avb<'a>(&'a self) -> Iter<'a, &'a AVBFlow, Self::INFO>;
    fn iter_tsn<'a>(&'a self) -> Iter<'a, &'a TSNFlow, Self::INFO>;
    fn iter<'a>(&'a self) -> Iter<'a, &'a FlowEnum, Self::INFO>;
    fn iter_avb_mut<'a>(&'a mut self) -> IterMut<'a, &'a AVBFlow, Self::INFO> {
        IterMut {
            iter: self.iter_avb(),
        }
    }
    fn iter_tsn_mut<'a>(&'a mut self) -> IterMut<'a, &'a TSNFlow, Self::INFO> {
        IterMut {
            iter: self.iter_tsn(),
        }
    }
}

/// 儲存的資料分為兩部份：資料流本身，以及隨附的資訊（T）。
///
/// __注意！這個資料結構 clone 的時候並不會把所有資料流複製一次，只會複製資訊的部份。__
///
/// 此處隱含的假設為：資料流本身不會時常變化，在演算法執行的過程中應該是唯一不變的，因此用一個 Rc 來記憶即可。
///
/// TODO 觀察在大資料量下這個改動是否有優化的效果。在小資料量下似乎沒啥差別。
#[derive(Clone)]
pub struct FlowTable<T: Clone + Eq> {
    arena: Rc<FlowArena>,
    infos: Vec<Option<T>>,
    avb_cnt: usize,
    tsn_cnt: usize,
}
impl<T: Clone + Eq> FlowTable<T> {
    pub fn new() -> Self {
        FlowTable {
            infos: vec![],
            arena: Rc::new(FlowArena::new()),
            avb_cnt: 0,
            tsn_cnt: 0,
        }
    }
    pub fn clone_as_type<U: Clone + Eq, F: Fn(FlowID, &T) -> U>(
        &self,
        transform: F,
    ) -> FlowTable<U> {
        let infos = self
            .infos
            .iter()
            .enumerate()
            .map(|(i, t)| t.as_ref().map(|t| transform(FlowID(i), t)))
            .collect();
        FlowTable {
            arena: self.arena.clone(),
            avb_cnt: self.avb_cnt,
            tsn_cnt: self.tsn_cnt,
            infos,
        }
    }
    pub fn apply_diff(&mut self, is_tsn: bool, other: &DiffFlowTable<T>) {
        if !self.is_same_flow_list(other) {
            panic!("試圖合併不相干的資料流表");
        }
        if is_tsn {
            for (flow, info) in other.iter_tsn() {
                self.update_info(flow.id, info.clone());
            }
        } else {
            for (flow, info) in other.iter_avb() {
                self.update_info(flow.id, info.clone());
            }
        }
    }
    pub fn insert(
        &mut self,
        tsns: Vec<TSNFlow>,
        avbs: Vec<AVBFlow>,
        default_info: T,
    ) -> Vec<FlowID> {
        let arena = Rc::get_mut(&mut self.arena).expect("插入資料流時發生數據爭用");
        let mut id_list = vec![];
        for flow in tsns.into_iter() {
            let id = arena.insert(flow);
            self.infos.push(Some(default_info.clone()));
            id_list.push(id);
            self.tsn_cnt += 1;
        }
        for flow in avbs.into_iter() {
            let id = arena.insert(flow);
            self.infos.push(Some(default_info.clone()));
            id_list.push(id);
            self.avb_cnt += 1;
        }
        id_list
    }
}
impl<T: Clone + Eq> IFlowTable for FlowTable<T> {
    type INFO = T;
    fn get_avb_cnt(&self) -> usize {
        self.avb_cnt
    }
    fn get_tsn_cnt(&self) -> usize {
        self.tsn_cnt
    }
    fn get_inner_arena(&self) -> &Rc<FlowArena> {
        &self.arena
    }
    fn get_info(&self, id: FlowID) -> Option<&T> {
        if id.0 < self.infos.len() {
            self.infos[id.0].as_ref()
        } else {
            None
        }
    }
    fn update_info(&mut self, id: FlowID, info: T) {
        if id.0 < self.infos.len() {
            self.infos[id.0] = Some(info);
        } else {
            panic!("更新資訊時越界");
        }
    }

    fn clone_as_diff(&self) -> DiffFlowTable<T> {
        DiffFlowTable::new(self)
    }
    fn iter_avb<'a>(&'a self) -> Iter<'a, &'a AVBFlow, T> {
        Iter::FlowTable {
            ptr: 0,
            cur_list: 0,
            id_lists: vec![&self.arena.avbs],
            flow_list: &self.arena.flow_list,
            infos: &self.infos,
            _marker: std::marker::PhantomData,
        }
    }
    fn iter_tsn<'a>(&'a self) -> Iter<'a, &'a TSNFlow, T> {
        Iter::FlowTable {
            ptr: 0,
            cur_list: 0,
            id_lists: vec![&self.arena.tsns],
            flow_list: &self.arena.flow_list,
            infos: &self.infos,
            _marker: std::marker::PhantomData,
        }
    }
    fn iter<'a>(&'a self) -> Iter<'a, &'a FlowEnum, T> {
        Iter::FlowTable {
            ptr: 0,
            cur_list: 0,
            id_lists: vec![&self.arena.tsns, &self.arena.avbs],
            flow_list: &self.arena.flow_list,
            infos: &self.infos,
            _marker: std::marker::PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct DiffFlowTable<T: Clone + Eq> {
    avb_diff: Vec<FlowID>,
    tsn_diff: Vec<FlowID>,
    table: FlowTable<OldNew<T>>,
}
impl<T: Clone + Eq> DiffFlowTable<T> {
    pub fn new(og_table: &FlowTable<T>) -> Self {
        DiffFlowTable {
            avb_diff: vec![],
            tsn_diff: vec![],
            table: FlowTable {
                arena: og_table.arena.clone(),
                infos: og_table
                    .infos
                    .iter()
                    .map(|opt| opt.as_ref().map(|t| OldNew::Old(t.clone())))
                    .collect(),
                avb_cnt: 0,
                tsn_cnt: 0,
            },
        }
    }
    /// 不管是否和本來相同，硬是更新
    pub fn update_info_force(&mut self, id: FlowID, info: T) {
        if !self.check_exist(id) {
            match self.table.get(id).unwrap() {
                FlowEnum::TSN(_) => self.tsn_diff.push(id),
                FlowEnum::AVB(_) => self.avb_diff.push(id),
            }
        }
        self.table.update_info(id, OldNew::New(info));
    }
}
impl<T: Clone + Eq> IFlowTable for DiffFlowTable<T> {
    type INFO = T;
    fn get_avb_cnt(&self) -> usize {
        self.avb_diff.len()
    }
    fn get_tsn_cnt(&self) -> usize {
        self.tsn_diff.len()
    }
    fn get_inner_arena(&self) -> &Rc<FlowArena> {
        &self.table.get_inner_arena()
    }
    fn get_info(&self, id: FlowID) -> Option<&Self::INFO> {
        if let Some(OldNew::New(info)) = self.table.get_info(id) {
            Some(info)
        } else {
            None
        }
    }
    fn update_info(&mut self, id: FlowID, info: Self::INFO) {
        if let OldNew::Old(og_value) = self.table.get_info(id).unwrap() {
            // NOTE: 若和本來值相同，就啥都不做
            if og_value == &info {
                return;
            }
            match self.table.get(id).unwrap() {
                FlowEnum::TSN(_) => self.tsn_diff.push(id),
                FlowEnum::AVB(_) => self.avb_diff.push(id),
            }
        }
        // NOTE: 如果本來就是 New，就不推進 diff 表（因為之前推過了）
        self.table.update_info(id, OldNew::New(info));
    }
    fn clone_as_diff(&self) -> DiffFlowTable<T> {
        self.clone()
    }

    fn iter_avb<'a>(&'a self) -> Iter<'a, &'a AVBFlow, T> {
        Iter::DiffTable {
            ptr: 0,
            cur_list: 0,
            id_lists: vec![&self.avb_diff],
            flow_list: &self.get_inner_arena().flow_list,
            infos: &self.table.infos,
            _marker: std::marker::PhantomData,
        }
    }
    fn iter_tsn<'a>(&'a self) -> Iter<'a, &'a TSNFlow, T> {
        Iter::DiffTable {
            ptr: 0,
            cur_list: 0,
            id_lists: vec![&self.tsn_diff],
            flow_list: &self.get_inner_arena().flow_list,
            infos: &self.table.infos,
            _marker: std::marker::PhantomData,
        }
    }
    fn iter<'a>(&'a self) -> Iter<'a, &'a FlowEnum, T> {
        Iter::DiffTable {
            ptr: 0,
            cur_list: 0,
            id_lists: vec![&self.tsn_diff, &self.avb_diff],
            flow_list: &self.get_inner_arena().flow_list,
            infos: &self.table.infos,
            _marker: std::marker::PhantomData,
        }
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

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum OldNew<T: Clone + Eq> {
    New(T),
    Old(T),
}

pub mod iter {
    use super::OldNew;
    use crate::utils::stream::{FlowEnum, FlowID};
    pub enum Iter<'a, F: From<&'a FlowEnum>, T: Clone + Eq> {
        FlowTable {
            ptr: usize,
            cur_list: usize,
            id_lists: Vec<&'a Vec<Option<FlowID>>>,
            flow_list: &'a Vec<Option<FlowEnum>>,
            infos: &'a Vec<Option<T>>,
            _marker: std::marker::PhantomData<&'a F>,
        },
        DiffTable {
            ptr: usize,
            cur_list: usize,
            id_lists: Vec<&'a Vec<FlowID>>,
            flow_list: &'a Vec<Option<FlowEnum>>,
            infos: &'a Vec<Option<OldNew<T>>>,
            _marker: std::marker::PhantomData<&'a F>,
        },
    }

    impl<'a, F: From<&'a FlowEnum>, T: Clone + Eq> Iterator for Iter<'a, F, T> {
        type Item = (F, &'a T);
        fn next(&mut self) -> Option<(F, &'a T)> {
            match self {
                Iter::FlowTable {
                    ref mut ptr,
                    ref mut cur_list,
                    flow_list,
                    id_lists,
                    infos,
                    ..
                } => {
                    while *cur_list < id_lists.len() {
                        let id_list = id_lists[*cur_list];
                        while *ptr < id_list.len() {
                            let cur_ptr = *ptr;
                            *ptr += 1;
                            if let Some(id) = id_list[cur_ptr] {
                                let flow = flow_list[id.0].as_ref().unwrap();
                                return Some((flow.into(), infos[id.0].as_ref().unwrap()));
                            }
                        }
                        *ptr = 0;
                        *cur_list += 1;
                    }
                }
                Iter::DiffTable {
                    ref mut ptr,
                    ref mut cur_list,
                    flow_list,
                    id_lists,
                    infos,
                    ..
                } => {
                    while *cur_list < id_lists.len() {
                        let id_list = id_lists[*cur_list];
                        if *ptr < id_list.len() {
                            let id = id_list[*ptr];
                            *ptr += 1;
                            let flow = flow_list[id.0].as_ref().unwrap();
                            if let Some(OldNew::New(info)) = infos[id.0].as_ref() {
                                return Some((flow.into(), info));
                            } else {
                                panic!("不知為何遍歷到沒東西的地方");
                            }
                        }
                        *ptr = 0;
                        *cur_list += 1;
                    }
                }
            }
            None
        }
    }

    pub struct IterMut<'a, F: From<&'a FlowEnum>, T: Clone + Eq> {
        pub iter: Iter<'a, F, T>,
    }

    impl<'a, F: From<&'a FlowEnum>, T: Clone + Eq> Iterator for IterMut<'a, F, T> {
        type Item = (F, &'a mut T);
        fn next(&mut self) -> Option<(F, &'a mut T)> {
            self.iter
                .next()
                .map(|(flow, t)| unsafe { (flow, &mut *(t as *const T as *mut T)) })
        }
    }
}
