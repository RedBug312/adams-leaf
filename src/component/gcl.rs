use crate::MAX_QUEUE;
use hashbrown::HashMap;
use num::integer::lcm;


#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
enum Entry {
    Port(usize, usize),
    Queue(usize, usize, u8),
}

type Events = Vec<(u32, u32, usize)>;

#[derive(Clone, Debug, Default)]
pub struct GateCtrlList {
    hyperperiod: u32,
    // TODO 這個資料結構有優化的空間
    events: HashMap<Entry, Events>,
    events_cache: HashMap<(usize, usize), Vec<(u32, u32)>>,
}


impl GateCtrlList {
    pub fn new(hyperperiod: u32) -> Self {
        Self { hyperperiod, ..Default::default() }
    }
    // XXX this function have never been called
    pub fn update_hyperperiod(&mut self, new_p: u32) {
        self.hyperperiod = lcm(self.hyperperiod, new_p);
    }
    pub fn clear(&mut self) {
        self.events = Default::default();
        self.events_cache = Default::default();
    }
    pub fn hyperperiod(&self) -> u32 {
        self.hyperperiod
    }
    /// 回傳 `link_id` 上所有閘門關閉事件。
    /// * `回傳值` - 一個陣列，其內容為 (事件開始時間, 事件持續時間);
    pub fn get_gate_events(&self, ends: (usize, usize)) -> &Vec<(u32, u32)> {
        // assert!(self.gate_evt.len() > link_id, "GCL: 指定了超出範圍的邊");
        let cache = self.events_cache.get(&ends);
        if cache.is_none() {
            // 生成快速查找表
            let mut lookup = Vec::<(u32, u32)>::new();
            let empty = vec![];
            let port = Entry::Port(ends.0, ends.1);
            let events = self.events.get(&port)
                .unwrap_or(&empty);
            let len = events.len();
            if len > 0 {
                let first_evt = self.events.get(&port).unwrap()[0];
                let mut cur_evt = (first_evt.0, first_evt.1);
                for &(start, duration, ..) in self.events.get(&port).unwrap()[1..len].iter() {
                    if cur_evt.0 + cur_evt.1 == start {
                        // 首尾相接
                        cur_evt.1 += duration; // 把閘門事件延長
                    } else {
                        lookup.push(cur_evt);
                        cur_evt = (start, duration);
                    }
                }
                lookup.push(cur_evt);
            }
            unsafe {
                // NOTE 內部可變，因為這只是加速用的
                let _self = self as *const Self as *mut Self;
                (*_self).events_cache.insert(ends, lookup);
            }
        }
        self.events_cache.get(&ends).as_ref().unwrap()
    }
    pub fn insert_gate_evt(
        &mut self,
        ends: (usize, usize),
        flow_id: usize,
        _queue_id: u8,
        start_time: u32,
        duration: u32,
    ) {
        self.events_cache.remove(&ends);
        let port = Entry::Port(ends.0, ends.1);
        let event = (start_time, duration, flow_id);
        let evts = &mut self.events.entry(port)
            .or_insert(Default::default());
        match evts.binary_search(&event) {
            Ok(_) => panic!("插入重複的閘門事件: link={}, {:?}", ends.0, event),
            Err(pos) => {
                if pos > 0 && evts[pos - 1].0 + evts[pos - 1].1 > start_time {
                    // 開始時間位於前一個事件中
                    panic!(
                        "插入重疊的閘門事件： link={}, {:?} v.s. {:?}",
                        ends.1,
                        evts[pos - 1],
                        event
                    );
                } else {
                    evts.insert(pos, event)
                }
            }
        }
    }
    pub fn insert_queue_evt(
        &mut self,
        ends: (usize, usize),
        flow_id: usize,
        queue_id: u8,
        start_time: u32,
        duration: u32,
    ) {
        if duration == 0 {
            return;
        }
        let event = (start_time, duration, flow_id);
        let queue = Entry::Queue(ends.0, ends.1, queue_id);
        let evts = &mut self.events.entry(queue)
            .or_insert(Default::default());
        match evts.binary_search(&event) {
            // FIXME: 這個異常有機率發生，試著重現看看！
            Ok(_) => panic!(
                "插入重複的佇列事件: link={}, queue={}, {:?}",
                ends.0, queue_id, event
            ),
            Err(pos) => {
                if pos > 0 && evts[pos - 1].0 + evts[pos - 1].1 >= start_time {
                    // 開始時間位於前一個事件中，則延伸前一個事件
                    evts[pos - 1].1 = start_time + duration - evts[pos - 1].0;
                } else {
                    evts.insert(pos, event)
                }
            }
        }
    }
    /// 會先確認 start~(start+duration) 這段時間中有沒有與其它事件重疊
    ///
    /// 若否，則回傳 None，應可直接塞進去。若有重疊，則會告知下一個空的時間（但不一定塞得進去）
    pub fn get_next_empty_time(&self, ends: (usize, usize), start: u32, duration: u32) -> Option<u32> {
        let s1 = self.get_next_spot(ends, start);
        let s2 = self.get_next_spot(ends, start + duration);
        if s1.0 != s2.0 {
            // 是不同的閘門事
            Some(s1.0)
        } else if s1.1 {
            // 是同一個閘門事件的開始
            None
        } else {
            // 是同一個閘門事件的結束，代表 start~duration 這段時間正處於該事件之中，重疊了!
            Some(s2.0)
        }
    }
    /// 計算最近的下一個「時間點」，此處的時間點有可能是閘門事件的開啟或結束。
    ///
    /// 回傳一組資料(usize, bool)，前者代表時間，後者代表該時間是閘門事件的開始還是結束（真代表開始）
    fn get_next_spot(&self, ends: (usize, usize), time: u32) -> (u32, bool) {
        // TODO 應該用二元搜索來優化?
        let empty = vec![];
        let port = Entry::Port(ends.0, ends.1);
        let evts = self.events.get(&port)
            .unwrap_or(&empty);
        for &(start, duration, ..) in evts {
            if start > time {
                return (start, true);
            } else if start + duration > time {
                return (start + duration, false);
            }
        }
        (self.hyperperiod, true)
    }
    /// 回傳 None 者，代表當前即是空的
    pub fn get_next_queue_empty_time(
        &self,
        ends: (usize, usize),
        queue_id: u8,
        time: u32,
    ) -> Option<u32> {
        let empty = vec![];
        let queue = Entry::Queue(ends.0, ends.1, queue_id);
        let evts = &self.events.get(&queue)
            .unwrap_or(&empty);
        for &(start, duration, _) in evts.iter() {
            if start <= time {
                if start + duration > time {
                    return Some(start + duration);
                } else {
                    return None;
                }
            }
        }
        None
    }
    pub fn delete_flow(&mut self, links: &Vec<(usize, usize)>, flow_id: usize) {
        for &ends in links {
            self.events_cache.remove(&ends);
            let port = Entry::Port(ends.0, ends.1);
            let gate_evt = &mut self.events.entry(port)
                .or_insert(Default::default());
            let mut i = 0;
            while i < gate_evt.len() {
                if gate_evt[i].2 == flow_id {
                    gate_evt.remove(i);
                } else {
                    i += 1;
                }
            }
            for queue_id in 0..MAX_QUEUE {
                let queue = Entry::Queue(ends.0, ends.1, queue_id);
                let queue_evt = &mut self.events.entry(queue)
                    .or_insert(Default::default());
                let mut i = 0;
                while i < queue_evt.len() {
                    if queue_evt[i].2 == flow_id {
                        queue_evt.remove(i);
                    } else {
                        i += 1;
                    }
                }
            }
        }
    }
}
