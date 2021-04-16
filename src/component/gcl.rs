use crate::{MAX_QUEUE, network::{EdgeIndex, Network}};
use num::integer::lcm;
use std::ops::Range;


#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
enum Entry {
    Port(EdgeIndex),
    Queue(EdgeIndex, u8),
}

impl Entry {
    fn index(&self) -> usize {
        match self {
            Entry::Port(ix) => 9 * ix.index(),
            Entry::Queue(ix, q) if *q < 8 => 9 * ix.index() + *q as usize + 1,
            Entry::Queue(..) => unreachable!(),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Event {
    stream: usize,
    window: Range<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct GateCtrlList {
    hyperperiod: u32,
    events: Vec<Vec<Event>>,
    // FIXME there's penalty 62ms -> 69ms when change cache key type to Entry
    events_cache: Vec<Option<Vec<Range<u32>>>>,
}


impl GateCtrlList {
    pub fn new(network: &Network, hyperperiod: u32) -> Self {
        let edge_count = network.edge_count();
        let events = vec![vec![]; edge_count * 9];
        let events_cache = vec![None; edge_count * 9];
        Self { hyperperiod, events, events_cache }
    }
    // XXX this function have never been called
    pub fn update_hyperperiod(&mut self, new_p: u32) {
        self.hyperperiod = lcm(self.hyperperiod, new_p);
    }
    pub fn clear(&mut self) {
        self.events.iter_mut().for_each(|e| e.clear());
        self.events_cache.iter_mut().for_each(|e| *e = None);
    }
    pub fn hyperperiod(&self) -> u32 {
        self.hyperperiod
    }
    fn events(&self, entry: Entry) -> &Vec<Event> {
        &self.events[entry.index()]
    }
    fn events_mut(&mut self, entry: Entry) -> &mut Vec<Event> {
        &mut self.events[entry.index()]
    }
    /// 回傳 `link_id` 上所有閘門關閉事件。
    /// * `回傳值` - 一個陣列，其內容為 (事件開始時間, 事件結束時間);
    pub fn get_gate_events(&self, edge: EdgeIndex) -> &Vec<Range<u32>> {
        // assert!(self.gate_evt.len() > link_id, "GCL: 指定了超出範圍的邊");
        let port = Entry::Port(edge);
        let cache = &self.events_cache[port.index()];
        if cache.is_none() {
            // 生成快速查找表
            let mut lookup = Vec::new();
            let port = Entry::Port(edge);
            let events = self.events(port);
            let len = events.len();
            if len > 0 {
                let first_evt = &events[0];
                let mut cur_evt = first_evt.window.clone();
                for event in events[1..len].iter() {
                    if cur_evt.end == event.window.start {
                        // 首尾相接
                        cur_evt.end = event.window.end; // 把閘門事件延長
                    } else {
                        lookup.push(cur_evt);
                        cur_evt = event.window.clone();
                    }
                }
                lookup.push(cur_evt);
            }
            unsafe {
                // NOTE 內部可變，因為這只是加速用的
                let _self = self as *const Self as *mut Self;
                (*_self).events_cache[port.index()] = Some(lookup);
            }
        }
        cache.as_ref().unwrap()
    }
    pub fn insert_gate_evt(
        &mut self,
        edge: EdgeIndex,
        tsn: usize,
        window: Range<u32>,
    ) {
        let port = Entry::Port(edge);
        self.events_cache[port.index()] = None;
        let event = Event::new(tsn, window);
        let evts = self.events_mut(port);
        match evts.binary_search_by_key(&event.window.start, |e| e.window.start) {
            Ok(_) => panic!("插入重複的閘門事件: link={:?}, {:?}", edge, event),
            Err(pos) => {
                if pos > 0 && evts[pos - 1].window.end > event.window.start {
                    // 開始時間位於前一個事件中
                    panic!(
                        "插入重疊的閘門事件： link={:?}, {:?} v.s. {:?}",
                        edge,
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
        edge: EdgeIndex,
        que: u8,
        tsn: usize,
        window: Range<u32>,
    ) {
        if window.start == window.end { return; }
        let event = Event::new(tsn, window);
        let queue = Entry::Queue(edge, que);
        let evts = self.events_mut(queue);
        match evts.binary_search_by_key(&event.tuple(), |e| e.tuple()) {
            // FIXME: 這個異常有機率發生，試著重現看看！
            Ok(_) => panic!(
                "插入重複的佇列事件: link={:?}, queue={}, {:?}",
                edge, que, event
            ),
            Err(pos) => {
                if pos > 0 && evts[pos - 1].window.end >= event.window.start {
                    // FIXME don't extend event, just panic
                    // 開始時間位於前一個事件中，則延伸前一個事件
                    evts[pos - 1].window.end = event.window.end;
                } else {
                    evts.insert(pos, event)
                }
            }
        }
    }
    /// 會先確認 start~(start+duration) 這段時間中有沒有與其它事件重疊
    ///
    /// 若否，則回傳 None，應可直接塞進去。若有重疊，則會告知下一個空的時間（但不一定塞得進去）
    pub fn get_next_empty_time(&self, edge: EdgeIndex, start: u32, duration: u32) -> Option<u32> {
        let s1 = self.get_next_spot(edge, start);
        let s2 = self.get_next_spot(edge, start + duration);
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
    fn get_next_spot(&self, edge: EdgeIndex, time: u32) -> (u32, bool) {
        // TODO 應該用二元搜索來優化?
        let port = Entry::Port(edge);
        let evts = self.events(port);
        for event in evts {
            if event.window.start > time {
                return (event.window.start, true);
            } else if event.window.end > time {
                return (event.window.end, false);
            }
        }
        (self.hyperperiod, true)
    }
    /// 回傳 None 者，代表當前即是空的
    pub fn get_next_queue_empty_time(
        &self,
        edge: EdgeIndex,
        queue_id: u8,
        time: u32,
    ) -> Option<u32> {
        let queue = Entry::Queue(edge, queue_id);
        let evts = self.events(queue);
        for event in evts {
            if event.window.start <= time {
                if event.window.end > time {
                    return Some(event.window.end);
                } else {
                    return None;
                }
            }
        }
        None
    }
    pub fn remove(&mut self, edge: EdgeIndex, tsn: usize) {
        let port = Entry::Port(edge);
        self.events_cache[port.index()] = None;
        let gate_evt = self.events_mut(port);
        let mut i = 0;
        while i < gate_evt.len() {
            if gate_evt[i].stream == tsn {
                gate_evt.remove(i);
            } else {
                i += 1;
            }
        }
        for queue_id in 0..MAX_QUEUE {
            let queue = Entry::Queue(edge, queue_id);
            let queue_evt = self.events_mut(queue);
            let mut i = 0;
            while i < queue_evt.len() {
                if queue_evt[i].stream == tsn {
                    queue_evt.remove(i);
                } else {
                    i += 1;
                }
            }
        }
    }
}

impl Event {
    fn new(stream: usize, window: Range<u32>) -> Self {
        Event { stream, window }
    }
    fn tuple(&self) -> (u32, u32, usize) {
        (self.window.start, self.window.end, self.stream)
    }
}
