use itertools::Itertools;
use crate::{MAX_QUEUE, network::{EdgeIndex, Network}};
use num::integer::lcm;
use std::{iter, ops::Range};
use super::base::intervalmap::IntervalMap;


#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Entry {
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

pub type Event = (Range<u32>, usize);

#[derive(Clone, Debug, Default)]
pub struct GateCtrlList {
    hyperperiod: u32,
    events: Vec<IntervalMap>,
}

impl GateCtrlList {
    pub fn new(network: &Network, hyperperiod: u32) -> Self {
        let edge_count = network.edge_count();
        let events = vec![IntervalMap::new(); edge_count * 9];
        Self { hyperperiod, events }
    }
    // XXX this function have never been called
    pub fn update_hyperperiod(&mut self, new_p: u32) {
        self.hyperperiod = lcm(self.hyperperiod, new_p);
    }
    pub fn clear(&mut self) {
        self.events.iter_mut().for_each(|e| e.clear());
    }
    pub fn hyperperiod(&self) -> u32 {
        self.hyperperiod
    }
    pub fn events(&self, entry: Entry) -> &Vec<Event> {
        self.events[entry.index()].intervals()
    }
    pub fn insert(&mut self, entry: Entry, tsn: usize, window: Range<u32>, period: u32) {
        let hyperperiod = self.hyperperiod;
        debug_assert!(window.end <= hyperperiod);
        let intmap = &mut self.events[entry.index()];
        (0..hyperperiod).step_by(period as usize)
            .map(|offset| shift(&window, offset))
            .for_each(|inst| intmap.insert(inst, tsn));
    }
    pub fn remove(&mut self, edge: EdgeIndex, tsn: usize) {
        let port = Entry::Port(edge);
        let intmap = &mut self.events[port.index()];
        intmap.remove_value(tsn);
        for q in 0..MAX_QUEUE {
            let queue = Entry::Queue(edge, q);
            let intmap = &mut self.events[queue.index()];
            intmap.remove_value(tsn);
        }
    }
    pub fn check_vacant_once(&self, entry: Entry, tsn: usize, window: Range<u32>) -> bool {
        let hyperperiod = self.hyperperiod;
        if window.end > hyperperiod { return false; }
        let intmap = &self.events[entry.index()];
        intmap.check_vacant(window, tsn)
    }
    pub fn check_vacant(&self, entry: Entry, tsn: usize, window: Range<u32>, period: u32) -> bool {
        let hyperperiod = self.hyperperiod;
        if window.end > period { return false; }
        if window.end > hyperperiod { return false; }
        (0..hyperperiod).step_by(period as usize)
            .map(|offset| shift(&window, offset))
            .all(|inst| self.check_vacant_once(entry, tsn, inst))
    }
    pub fn query_later_vacant_once(&self, entry: Entry, tsn: usize, window: Range<u32>) -> Option<u32> {
        if self.check_vacant_once(entry, tsn, window.clone()) { return Some(0); }
        let hyperperiod = self.hyperperiod;
        let intmap = &self.events[entry.index()];
        if window.end > hyperperiod { return None; }

        let padded = (hyperperiod..hyperperiod, usize::MAX);
        let afters = intmap.intervals_after(window.start);
        let afters = afters.iter().chain(iter::once(&padded));
        afters.tuple_windows()
            .map(|(prev, next)| prev.0.end..next.0.start)
            .find(|vacant| vacant.end - vacant.start >= window.end - window.start)
            .map(|vacant| vacant.start - window.start)
    }
    pub fn query_later_vacant(&self, entry: Entry, tsn: usize, window: Range<u32>, period: u32) -> Option<u32> {
        debug_assert!(window.end <= period);
        let hyperperiod = self.hyperperiod;
        if window.end > hyperperiod { return None; }
        let mut result = 0;
        while !self.check_vacant(entry, tsn, shift(&window, result), period) {
            let increment = (0..hyperperiod).step_by(period as usize)
                .map(|offset| shift(&window, result + offset))
                .map(|inst| self.query_later_vacant_once(entry, tsn, inst))
                .try_fold(0, try_max)?;
            debug_assert!(increment > 0);
            result += increment;
        }
        Some(result)
    }
}

#[inline]
fn try_max(x: u32, y: Option<u32>) -> Option<u32> {
    y.map(|y| u32::max(x, y))
}

#[inline]
fn shift(window: &Range<u32>, offset: u32) -> Range<u32> {
    (window.start + offset)..(window.end + offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> GateCtrlList {
        let mut network = Network::new();
        network.add_nodes(2, 0);
        network.add_edges(vec![(0, 1, 1000.0)]);
        let mut gcl = GateCtrlList::new(&network, 10);
        let entry = Entry::Port(EdgeIndex::new(0));
        gcl.insert(entry, 0, 0..1, 5);
        gcl.insert(entry, 0, 2..3, 5);
        gcl.insert(entry, 1, 3..4, 10);
        gcl.insert(entry, 2, 6..7, 10);
        let before = [(0..1, 0), (2..3, 0), (3..4, 1), (5..6, 0), (6..7, 2), (7..8, 0)];
        assert_eq!(gcl.events(entry), &before);
        gcl
    }

    #[test]
    fn it_checks_vacant() {
        let gcl = setup();
        let entry = Entry::Port(EdgeIndex::new(0));
        // before: 0 - 0 1 - 0 2 0 - -
        // expect: 0 - 0 1 + 0 2 0 - +
        assert_eq!(gcl.check_vacant(entry, 9, 1..2, 5), false);
        assert_eq!(gcl.check_vacant(entry, 9, 4..5, 5), true);
    }

    #[test]
    fn it_queries_later_vacant_once() {
        let mut gcl = setup();
        gcl.clear();
        let entry = Entry::Port(EdgeIndex::new(0));
        gcl.insert(entry, 0, 0..1, 5);
        gcl.insert(entry, 1, 2..3, 10);
        let before = [(0..1, 0), (2..3, 1), (5..6, 0)];
        assert_eq!(gcl.events(entry), &before);
        // before: 0 - 1 - - 0 - - - -
        // expect: 0 - 1 - - 0 + + + -
        assert_eq!(gcl.query_later_vacant_once(entry, 9, 0..2), Some(3));
        assert_eq!(gcl.query_later_vacant_once(entry, 9, 1..4), Some(5));
        assert_eq!(gcl.query_later_vacant_once(entry, 9, 0..5), None);

        gcl.clear();
        gcl.insert(entry, 0, 0..2, 10);
        gcl.insert(entry, 0, 2..5, 10);
        let before = [(0..5, 0)];
        assert_eq!(gcl.events(entry), &before);
        // before: 0 0 0 0 0 - - - - -
        // expect: 0 0 0 0 0 + + + - -
        assert_eq!(gcl.query_later_vacant_once(entry, 9, 0..3), Some(5));
        assert_eq!(gcl.query_later_vacant_once(entry, 9, 2..5), Some(3));
        assert_eq!(gcl.query_later_vacant_once(entry, 9, 4..7), Some(1));
        assert_eq!(gcl.query_later_vacant_once(entry, 9, 6..9), Some(0));
    }

    #[test]
    fn it_queries_later_vacant() {
        let gcl = setup();
        let entry = Entry::Port(EdgeIndex::new(0));
        // before: 0 - 0 1 - 0 2 0 - -
        // expect: 0 - 0 1 + 0 2 0 - +
        assert_eq!(gcl.query_later_vacant(entry, 9, 0..1, 5), Some(4));
        assert_eq!(gcl.query_later_vacant(entry, 9, 2..3, 5), Some(2));
        assert_eq!(gcl.query_later_vacant(entry, 9, 0..2, 5), None);
        assert_eq!(gcl.query_later_vacant(entry, 9, 2..4, 5), None);
    }
}
