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

pub type Event = (Range<u32>, usize);

#[derive(Clone, Debug, Default)]
pub struct GateCtrlList {
    hyperperiod: u32,
    events: HashMap<Entry, IntervalMap>,
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
    }
    pub fn hyperperiod(&self) -> u32 {
        self.hyperperiod
    }
    pub fn events(&self, entry: Entry) -> &Vec<Event> {
        static EMPTY: Vec<Event> = vec![];
        self.events.get(&entry)
            .map_or(&EMPTY, |m| m.intervals())
    }
    pub fn remove(&mut self, ends: &(usize, usize), tsn: usize) {
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
        let hyperperiod = self.hyperperiod();
        debug_assert!(window.end <= hyperperiod);
        let events = self.events.entry(entry)
            .or_insert_with(|| IntervalMap::new());
        (0..hyperperiod).step_by(period as usize)
            .map(|offset| shift(&window, offset))
            .for_each(|inst| events.insert(inst, tsn))
    }
    fn check_idle_once(&self, entry: Entry, tsn: usize, window: Range<u32>) -> bool {
        let hyperperiod = self.hyperperiod();
        let events = self.events.get(&entry);
        if events.is_none() { return true; }
        let events = events.unwrap();
        window.end <= hyperperiod && events.check_available(window, tsn)
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
        if window.end > hyperperiod { return None; }
        let padded = (hyperperiod..hyperperiod, usize::MAX);
        let mut afters = Vec::from(events.intervals_after(window.start));
        afters.push(padded);
        afters.windows(2)
            // .inspect(|i| println!("{:?}", i))
            .map(|pair| pair[0].0.end..pair[1].0.start)
            .find(|idle| idle.end - idle.start >= window.end - window.start)
            .map(|idle| idle.start - window.start)
    }
    pub fn query_later_idle(&self, entry: Entry, tsn: usize, window: Range<u32>, period: u32) -> Option<u32> {
        debug_assert!(matches!(entry, Entry::Port(..)));
        debug_assert!(window.end <= period);
        let mut offset = 0;
        let hyperperiod = self.hyperperiod();
        if window.end > hyperperiod { return None; }
        while !self.check_idle(entry, tsn, shift(&window, offset), period) {
            let increment = (0..hyperperiod).step_by(period as usize)
                .map(|timeshift| shift(&window, timeshift + offset))
                .map(|inst| self.query_later_idle_once(entry, tsn, inst))
                // .inspect(|x|println!("{:?}", x))
                .collect::<Option<Vec<_>>>()
                .and_then(|incrs| incrs.into_iter().max())?;
            debug_assert!(increment > 0);
            offset += increment;
        }
        Some(offset)
    }
}

#[inline]
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
        let before = [(0..1, 0), (2..3, 0), (3..4, 1), (5..6, 0), (6..7, 2), (7..8, 0)];
        assert_eq!(gcl.events(entry), &before);
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
        let before = [(0..1, 0), (2..3, 1), (5..6, 0)];
        assert_eq!(gcl.events(entry), &before);
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
