use crate::MAX_QUEUE;
use hashbrown::HashMap;
use num::integer::lcm;
use std::ops::Range;
use super::base::intervalmap::IntervalMap;


#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Entry {
    Port(usize, usize),
    Queue(usize, usize, u8),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Event {
    pub stream: usize,
    pub window: Range<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct GateCtrlList {
    hyperperiod: u32,
    events: HashMap<Entry, IntervalMap>,
    // FIXME there's penalty 62ms -> 69ms when change cache key type to Entry
    events_cache: HashMap<(usize, usize), Vec<Range<u32>>>,
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
    pub fn events(&self, entry: Entry) -> Vec<Event> {
        static EMPTY: Vec<(Range<u32>, usize)> = vec![];
        self.events.get(&entry)
            .map(|m| m.intervals())
            .unwrap_or(&EMPTY)
            .iter()
            .map(|i| Event::new(i.1, i.0.clone()))
            .collect::<Vec<_>>()
    }
    /// 回傳 `link_id` 上所有閘門關閉事件。
    /// * `回傳值` - 一個陣列，其內容為 (事件開始時間, 事件結束時間);
    pub fn get_gate_events(&self, ends: (usize, usize)) -> &Vec<Range<u32>> {
        // assert!(self.gate_evt.len() > link_id, "GCL: 指定了超出範圍的邊");
        let cache = self.events_cache.get(&ends);
        if cache.is_none() {
            // 生成快速查找表
            let mut lookup = Vec::new();
            let port = Entry::Port(ends.0, ends.1);
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
                (*_self).events_cache.insert(ends, lookup);
            }
        }
        self.events_cache.get(&ends).as_ref().unwrap()
    }
    pub fn remove(&mut self, ends: &(usize, usize), tsn: usize) {
        self.events_cache.remove(&ends);
        let port = Entry::Port(ends.0, ends.1);
        let map = self.events.entry(port)
            .or_insert_with(|| IntervalMap::new());
        map.remove_value(tsn);
        for queue_id in 0..MAX_QUEUE {
            let queue = Entry::Queue(ends.0, ends.1, queue_id);
            let map = self.events.entry(queue)
                .or_insert_with(|| IntervalMap::new());
            map.remove_value(tsn);
        }
    }
    pub fn insert(&mut self, entry: Entry, tsn: usize, window: Range<u32>, period: u32) {
        self.events_cache.remove(&entry.ends());
        let hyperperiod = self.hyperperiod();
        let events = self.events.entry(entry)
            .or_insert_with(|| IntervalMap::new());
        (0..hyperperiod).step_by(period as usize)
            .map(|offset| shift(&window, offset))
            .for_each(|inst| match entry {
                Entry::Port(..)  => events.insert(inst, tsn),
                Entry::Queue(..) => events.extend(inst, tsn),
            })
    }
    fn check_idle_once(&self, entry: Entry, tsn: usize, window: Range<u32>) -> bool {
        let hyperperiod = self.hyperperiod();
        let events = self.events.get(&entry);
        if events.is_none() { return true; }
        let events = events.unwrap();
        window.end <= hyperperiod && match entry {
            Entry::Port(..)  => events.check_insertable(window),
            Entry::Queue(..) => events.check_extendable(window, tsn),
        }
    }
    pub fn check_idle(&self, entry: Entry, tsn: usize, window: Range<u32>, period: u32) -> bool {
        let hyperperiod = self.hyperperiod();
        if window.end > period { return false; }
        (0..hyperperiod).step_by(period as usize)
            .map(|offset| shift(&window, offset))
            .all(|inst| self.check_idle_once(entry, tsn, inst))
    }
    fn query_later_idle_once(&self, entry: Entry, tsn: usize, window: Range<u32>) -> Option<u32> {
        debug_assert!(matches!(entry, Entry::Port(..)));
        if self.check_idle_once(entry, tsn, window.clone()) { return Some(0) }
        let events = self.events.get(&entry);
        if events.is_none() { return Some(0); }
        let events = events.unwrap();

        let hyperperiod = self.hyperperiod();
        let padded = (hyperperiod..hyperperiod, usize::MAX);
        let mut afters = Vec::from(events.intervals_after(window.start));
        afters.push(padded);
        afters.windows(2)
            .map(|pair| pair[0].0.end..pair[1].0.start)
            .find(|idle| idle.end - idle.start >= window.end - window.start)
            .map(|idle| idle.start - window.start)
    }
    pub fn query_later_idle(&self, entry: Entry, tsn: usize, window: Range<u32>, period: u32) -> Option<u32> {
        debug_assert!(matches!(entry, Entry::Port(..)));
        debug_assert!(window.end <= period);
        let mut offset = 0;
        let hyperperiod = self.hyperperiod();
        while !self.check_idle(entry, tsn, shift(&window, offset), period) {
            let increment = (0..hyperperiod).step_by(period as usize)
                .map(|timeshift| shift(&window, timeshift + offset))
                .map(|inst| self.query_later_idle_once(entry, tsn, inst))
                .collect::<Option<Vec<_>>>()
                .and_then(|incrs| incrs.into_iter().max())?;
            debug_assert!(increment > 0);
            offset += increment;
        }
        Some(offset)
    }
}

impl Event {
    fn new(stream: usize, window: Range<u32>) -> Self {
        Event { stream, window }
    }
}

impl Entry {
    fn ends(&self) -> (usize, usize) {
        match self {
            Entry::Port(src, dst) => (*src, *dst),
            Entry::Queue(src, dst, _) => (*src, *dst),
        }
    }
}

fn shift(window: &Range<u32>, offset: u32) -> Range<u32> {
    (window.start + offset)..(window.end + offset)
}


#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> GateCtrlList {
        let mut gcl = GateCtrlList::new(10);
        let entry = Entry::Port(0, 1);
        gcl.insert(entry, 0, 0..1, 5);
        gcl.insert(entry, 0, 2..3, 5);
        gcl.insert(entry, 1, 3..4, 10);
        gcl.insert(entry, 2, 6..7, 10);
        let before = vec![
            Event::new(0, 0..1), Event::new(0, 2..3), Event::new(1, 3..4),
            Event::new(0, 5..6), Event::new(2, 6..7), Event::new(0, 7..8),
        ];
        assert_eq!(gcl.events(entry), before);
        gcl
    }

    #[test]
    fn it_checks_idle() {
        let gcl = setup();
        let entry = Entry::Port(0, 1);
        // before: 0 - 0 1 - 0 2 0 - -
        // expect: 0 - 0 1 + 0 2 0 - +
        assert_eq!(gcl.check_idle(entry, 9, 1..2, 5), false);
        assert_eq!(gcl.check_idle(entry, 9, 4..5, 5), true);
    }

    #[test]
    fn it_queries_later_idle_once() {
        let mut gcl = GateCtrlList::new(10);
        let entry = Entry::Port(0, 1);
        gcl.insert(entry, 0, 0..1, 5);
        gcl.insert(entry, 1, 2..3, 10);
        let before = vec![
            Event::new(0, 0..1), Event::new(1, 2..3), Event::new(0, 5..6),
        ];
        assert_eq!(gcl.events(entry), before);
        // before: 0 - 1 - - 0 - - - -
        // expect: 0 - 1 - - 0 + + + -
        assert_eq!(gcl.query_later_idle_once(entry, 9, 0..2), Some(3));
        assert_eq!(gcl.query_later_idle_once(entry, 9, 1..4), Some(5));
        assert_eq!(gcl.query_later_idle_once(entry, 9, 0..5), None);
    }

    #[test]
    fn it_queries_later_idle() {
        let gcl = setup();
        let entry = Entry::Port(0, 1);
        // before: 0 - 0 1 - 0 2 0 - -
        // expect: 0 - 0 1 + 0 2 0 - +
        assert_eq!(gcl.query_later_idle(entry, 9, 0..1, 5), Some(4));
        assert_eq!(gcl.query_later_idle(entry, 9, 2..3, 5), Some(2));
        assert_eq!(gcl.query_later_idle(entry, 9, 0..2, 5), None);
        assert_eq!(gcl.query_later_idle(entry, 9, 2..4, 5), None);
    }
}
