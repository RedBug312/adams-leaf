use std::ops::Range;
use std::collections::btree_map::BTreeMap;

type Pair = (Range<u32>, usize);

#[derive(Clone, Debug, Default)]
pub struct IntervalMap {
    inner: BTreeMap<u32, Pair>,
}

impl IntervalMap {
    pub fn new() -> Self {
        IntervalMap::default()
    }
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=&Pair> + 'a {
        self.inner.iter()
            .map(|node| node.1)
    }
    pub fn iter_after<'a>(&'a self, start: u32) -> impl Iterator<Item=&Pair> + 'a {
        let pred = self.inner.range(..=start).next_back();
        if pred.is_some() && pred.as_ref().unwrap().1.0.end >= start {
            let pair = pred.as_ref().unwrap().1;
            self.inner.range(pair.0.start..)
        } else {
            self.inner.range(start..)
        }.map(|node| node.1)
    }
    pub fn insert(&mut self, key: Range<u32>, value: usize) {
        debug_assert_ne!(value, usize::MAX);
        debug_assert_eq!(self.check_vacant(key.clone(), value), true);
        let pred = self.inner.range_mut(..=key.start).next_back();
        if pred.is_some() && pred.as_ref().unwrap().1.0.end >= key.start
                          && pred.as_ref().unwrap().1.1 == value {
            pred.map(|(_, mut pair)| {pair.0.end = key.end;});
        } else {
            self.inner.insert(key.start, (key, value));
        }
    }
    pub fn remove_value(&mut self, value: usize) {
        self.inner.retain(|_, pair| pair.1 != value)
    }
    pub fn clear(&mut self) {
        self.inner = BTreeMap::new();
    }
    pub fn check_vacant(&self, key: Range<u32>, value: usize) -> bool {
        let pred = self.inner.range(..=key.start).next_back();
        let succ = self.inner.range(key.start..).next();
        if succ.is_some() && key.end > succ.unwrap().1.0.start { false }
        else if pred.is_some() && pred.unwrap().1.0.end > key.start { pred.unwrap().1.1 == value }
        else { true }
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use super::*;

    fn setup() -> IntervalMap {
        let mut map = IntervalMap::new();
        map.insert(6..8, 1);
        map.insert(2..4, 0);
        assert_eq!(map.iter().collect_vec(), [&(2..4, 0), &(6..8, 1)]);
        map
    }

    #[test]
    fn it_checks_vacant() {
        let map = setup();
        let max = usize::MAX;
        assert_eq!(map.check_vacant(0..2, max), true);
        assert_eq!(map.check_vacant(4..6, max), true);
        assert_eq!(map.check_vacant(8..9, max), true);
        assert_eq!(map.check_vacant(0..3, max), false);
        assert_eq!(map.check_vacant(0..5, max), false);
        assert_eq!(map.check_vacant(0..9, max), false);
        assert_eq!(map.check_vacant(3..5, max), false);
        assert_eq!(map.check_vacant(3..7, max), false);
        assert_eq!(map.check_vacant(5..9, max), false);
    }

    #[test]
    fn it_connects_intervals() {
        let mut map = setup();
        map.insert(4..6, 1);
        map.insert(8..9, 1);
        map.insert(10..12, 1);
        let expect = [&(2..4, 0), &(4..6, 1), &(6..9, 1), &(10..12, 1)];
        assert_eq!(map.iter().collect_vec(), expect);
    }

    #[test]
    fn it_queries_intervals_after() {
        let map = setup();
        assert_eq!(map.iter_after(0).collect_vec(), [&(2..4, 0), &(6..8, 1)]);
        assert_eq!(map.iter_after(2).collect_vec(), [&(2..4, 0), &(6..8, 1)]);
        assert_eq!(map.iter_after(3).collect_vec(), [&(2..4, 0), &(6..8, 1)]);
        assert_eq!(map.iter_after(4).collect_vec(), [&(2..4, 0), &(6..8, 1)]);
        assert_eq!(map.iter_after(5).collect_vec(), [&(6..8, 1)]);
    }
}
