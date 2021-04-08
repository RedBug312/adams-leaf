use std::ops::Range;

#[derive(Clone, Debug, Default)]
pub struct IntervalMap {
    intervals: Vec<(Range<u32>, usize)>,
}

impl IntervalMap {
    pub fn new() -> Self {
        IntervalMap::default()
    }
    pub fn intervals(&self) -> &Vec<(Range<u32>, usize)> {
        &self.intervals
    }
    pub fn intervals_after(&self, start: u32) -> &[(Range<u32>, usize)] {
        match self.intervals.binary_search_by_key(&start, |i| i.0.start) {
            Ok(pos) => &self.intervals[pos..],
            Err(pos) => &self.intervals[pos..],
        }
    }
    pub fn insert(&mut self, key: Range<u32>, value: usize) {
        // TODO debug_assert is safe?
        // println!("{:?}", key);
        assert_eq!(self.check_insertable(key.clone()), true);
        match self.intervals.binary_search_by_key(&key.start, |i| i.0.start) {
            Ok(_) => unreachable!(),
            Err(pos) if self.pred_connected(pos, &key, value).is_some() => {
                self.intervals[pos - 1].0.end = key.end;
            }
            Err(pos) => {
                self.intervals.insert(pos, (key, value));
            }
        }
    }
    pub fn extend(&mut self, key: Range<u32>, value: usize) {
        // TODO debug_assert is safe?
        // println!("{:?}", key);
        assert_eq!(self.check_extendable(key.clone(), value), true);
        match self.intervals.binary_search_by_key(&key.start, |i| i.0.start) {
            Ok(_) => unreachable!(),
            Err(pos) if self.pred_connected_alt(pos, &key, value).is_some() => {
                self.intervals[pos - 1].0.end = key.end;
            }
            Err(pos) => {
                self.intervals.insert(pos, (key, value));
            }
        }
    }
    pub fn remove_value(&mut self, value: usize) {
        self.intervals.retain(|i| i.1 != value)
    }
    pub fn check_insertable(&self, key: Range<u32>) -> bool {
        match self.intervals.binary_search_by_key(&key.start, |i| i.0.start) {
            Ok(_) => false,
            Err(pos) if self.succ_conflicted(pos, &key).is_some() => false,
            Err(pos) if self.pred_conflicted(pos, &key).is_some() => false,
            Err(_) => true,
        }
    }
    pub fn check_extendable(&self, key: Range<u32>, value: usize) -> bool {
        match self.intervals.binary_search_by_key(&key.start, |i| i.0.start) {
            Ok(_) => false,
            Err(pos) if self.succ_conflicted(pos, &key).is_some() => false,
            Err(pos) if self.pred_conflicted(pos, &key) == Some(value) => true,
            Err(pos) if self.pred_conflicted(pos, &key).is_some() => false,
            Err(_) => true,
        }
    }
    fn pred_connected(&self, pos: usize, key: &Range<u32>, value: usize) -> Option<usize> {
        match pos > 0 && self.intervals[pos - 1].0.end == key.start
                      && self.intervals[pos - 1].1 == value {
            true => Some(self.intervals[pos - 1].1),
            false => None,
        }
    }
    fn pred_connected_alt(&self, pos: usize, key: &Range<u32>, value: usize) -> Option<usize> {
        match pos > 0 && self.intervals[pos - 1].0.end >= key.start
                      && self.intervals[pos - 1].1 == value {
            true => Some(self.intervals[pos - 1].1),
            false => None,
        }
    }
    fn pred_conflicted(&self, pos: usize, key: &Range<u32>) -> Option<usize> {
        match pos > 0 && self.intervals[pos - 1].0.end > key.start {
            true => Some(self.intervals[pos - 1].1),
            false => None,
        }
    }
    fn succ_conflicted(&self, pos: usize, key: &Range<u32>) -> Option<usize> {
        let len = self.intervals.len();
        match pos < len && key.end > self.intervals[pos].0.start {
            true => Some(self.intervals[pos].1),
            false => None,
        }
    }
}

#[cfg(test)]
mod tests2 {
    use super::*;

    fn setup() -> IntervalMap {
        let mut map = IntervalMap::new();
        map.insert(6..8, 1);
        map.insert(2..4, 0);
        assert_eq!(map.intervals, vec![(2..4, 0), (6..8, 1)]);
        map
    }

    #[test]
    fn it_checks_insertable() {
        let map = setup();
        assert_eq!(map.check_insertable(0..2), true);
        assert_eq!(map.check_insertable(4..6), true);
        assert_eq!(map.check_insertable(8..9), true);
        assert_eq!(map.check_insertable(0..3), false);
        assert_eq!(map.check_insertable(0..5), false);
        assert_eq!(map.check_insertable(0..9), false);
        assert_eq!(map.check_insertable(3..5), false);
        assert_eq!(map.check_insertable(3..7), false);
        assert_eq!(map.check_insertable(5..9), false);
    }

    #[test]
    fn it_connects_intervals() {
        let mut map = setup();
        map.insert(4..6, 1);
        map.insert(8..9, 1);
        map.insert(10..12, 1);
        let expect = vec![(2..4, 0), (4..6, 1), (6..9, 1), (10..12, 1)];
        assert_eq!(map.intervals, expect);
    }

    #[test]
    fn it_queries_intervals_after() {
        let map = setup();
        assert_eq!(map.intervals_after(0), &[(2..4, 0), (6..8, 1)]);
        assert_eq!(map.intervals_after(2), &[(2..4, 0), (6..8, 1)]);
        assert_eq!(map.intervals_after(3), &[(6..8, 1)]);
        assert_eq!(map.intervals_after(4), &[(6..8, 1)]);
    }
}

// fn is_conflicted(pred: Range<u32>, succ: Range<u32>) -> bool {
//     pred.end > succ.start
// }
