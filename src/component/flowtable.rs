use crate::utils::stream::{AVB, TSN};


const KTH_DEFAULT: usize = 0;

#[derive(Clone)]
enum Choice {
    Pending(usize),
    Stay(usize),
    Switch(usize, usize),
}

enum Either {
    TSN(usize, TSN),
    AVB(usize, AVB),
}

#[derive(Default)]
pub struct FlowArena {
    pub avbs: Vec<usize>,
    pub tsns: Vec<usize>,
    streams: Vec<Either>,
}

#[derive(Clone)]
pub struct FlowTable {
    choices: Vec<Choice>,
    pub avb_diff: Vec<usize>,
    pub tsn_diff: Vec<usize>,
}


impl FlowArena {
    pub fn new() -> Self {
        FlowArena { ..Default::default() }
    }
    pub fn append(&mut self, tsns: Vec<TSN>, avbs: Vec<AVB>) -> (Vec<usize>, Vec<usize>) {
        let mut new_tsns = vec![];
        let len = self.streams.len();
        for (idx, tsn) in tsns.into_iter().enumerate() {
            self.tsns.push(len + idx);
            new_tsns.push(len + idx);
            self.streams.push(Either::TSN(len + idx, tsn));
        }
        let mut new_avbs = vec![];
        let len = self.streams.len();
        for (idx, avb) in avbs.into_iter().enumerate() {
            self.avbs.push(len + idx);
            new_avbs.push(len + idx);
            self.streams.push(Either::AVB(len + idx, avb));
        }
        (new_tsns, new_avbs)
    }
    pub fn tsn(&self, id: usize) -> Option<&TSN> {
        let either = self.streams.get(id)
            .expect("Failed to obtain TSN spec from an invalid id");
        match either {
            Either::TSN(_, spec) => Some(spec),
            Either::AVB(_, _) => None,
        }
    }
    pub fn avb(&self, id: usize) -> Option<&AVB> {
        let either = self.streams.get(id)
            .expect("Failed to obtain AVB spec from an invalid id");
        match either {
            Either::TSN(_, _) => None,
            Either::AVB(_, spec) => Some(spec),
        }
    }
    pub fn ends(&self, id: usize) -> (usize, usize) {
        let either = self.streams.get(id)
            .expect("Failed to obtain end devices from an invalid id");
        match either {
            Either::TSN(_, tsn) => (tsn.src, tsn.dst),
            Either::AVB(_, avb) => (avb.src, avb.dst),
        }
    }
    // XXX
    pub fn is_tsn(&self, id: usize) -> bool {
        let either = self.streams.get(id)
            .expect("Failed to obtain end devices from an invalid id");
        matches!(either, Either::TSN(_, _))
    }
    pub fn len(&self) -> usize {
        self.streams.len()
    }
}

impl FlowTable {
    pub fn new() -> Self {
        FlowTable {
            choices: vec![],
            avb_diff: vec![],
            tsn_diff: vec![],
        }
    }
    pub fn resize(&mut self, len: usize) {
        // for oldnewtable
        self.choices.resize(len, Choice::Pending(KTH_DEFAULT));
    }
    pub fn apply(&mut self, is_tsn: bool) {
        if is_tsn {
            for id in self.tsn_diff.drain(..) {
                self.choices[id].confirm();
            }
        } else {
            for id in self.avb_diff.drain(..) {
                self.choices[id].confirm();
            }
        }
    }
    pub fn update_info(&mut self, id: usize, info: usize) {
        debug_assert!(id < self.choices.len());
        self.choices[id] = Choice::Stay(info);
    }
}

impl FlowTable {
    /// 不管是否和本來相同，硬是更新
    pub fn update_tsn_info_force_diff(&mut self, id: usize, info: usize) {
        self.tsn_diff.push(id);
        self.choices[id].pick(info);
    }
    pub fn update_avb_info_force_diff(&mut self, id: usize, info: usize) {
        self.avb_diff.push(id);
        self.choices[id].pick(info);
    }
    pub fn update_tsn_info_diff(&mut self, id: usize, info: usize) {
        // FIXME: allow choice switch(x, x) for simplicity
        if let Some(Choice::Stay(og_value)) = self.choices.get(id) {
            // NOTE: 若和本來值相同，就啥都不做
            if *og_value == info {
                return;
            }
            self.tsn_diff.push(id);
        }
        // NOTE: 如果本來就是 New，就不推進 diff 表（因為之前推過了）
        self.choices[id].pick(info);
    }
    pub fn kth_prev(&self, id: usize) -> Option<usize> {
        self.choices[id].kth_prev()
    }
    pub fn kth_next(&self, id: usize) -> Option<usize> {
        self.choices[id].kth_next()
    }
    pub fn filter_pending<'a>(&'a self, source: &'a Vec<usize>)
        -> impl Iterator<Item=usize> + 'a {
        source.iter().cloned()
            .filter(move |&id| matches!(self.choices[id],
                    Choice::Pending(_))
        )
    }
    pub fn filter_switch<'a>(&'a self, source: &'a Vec<usize>)
        -> impl Iterator<Item=usize> + 'a {
        source.iter().cloned()
            .filter(move |&id| matches!(self.choices[id],
                    Choice::Switch(prev, next) if prev != next)
        )
    }
}


impl Choice {
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
    fn kth_prev(&self) -> Option<usize> {
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
