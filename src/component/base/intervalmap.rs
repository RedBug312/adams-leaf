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
        debug_assert_ne!(value, usize::MAX);
        assert_eq!(self.check_available(key.clone(), value), true);
        match self.intervals.binary_search_by_key(&key.start, |i| i.0.start) {
            Ok(_) => unreachable!(),
            Err(pos) if self.pred_connected(pos, &key) == Some(value) => {
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
    pub fn check_available(&self, key: Range<u32>, value: usize) -> bool {
        match self.intervals.binary_search_by_key(&key.start, |i| i.0.start) {
            Ok(_) => false,
            Err(pos) if self.succ_conflicted(pos, &key).is_some() => false,
            Err(pos) if self.pred_conflicted(pos, &key) == Some(value) => true,
            Err(pos) if self.pred_conflicted(pos, &key).is_some() => false,
            Err(_) => true,
        }
    }
    fn pred_connected(&self, pos: usize, key: &Range<u32>) -> Option<usize> {
        match pos > 0 && self.intervals[pos - 1].0.end >= key.start {
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
mod tests {
    use super::*;

    fn setup() -> IntervalMap {
        let mut map = IntervalMap::new();
        map.insert(6..8, 1);
        map.insert(2..4, 0);
        assert_eq!(map.intervals, vec![(2..4, 0), (6..8, 1)]);
        map
    }

    #[test]
    fn it_checks_available() {
        let map = setup();
        let max = usize::MAX;
        assert_eq!(map.check_available(0..2, max), true);
        assert_eq!(map.check_available(4..6, max), true);
        assert_eq!(map.check_available(8..9, max), true);
        assert_eq!(map.check_available(0..3, max), false);
        assert_eq!(map.check_available(0..5, max), false);
        assert_eq!(map.check_available(0..9, max), false);
        assert_eq!(map.check_available(3..5, max), false);
        assert_eq!(map.check_available(3..7, max), false);
        assert_eq!(map.check_available(5..9, max), false);
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
