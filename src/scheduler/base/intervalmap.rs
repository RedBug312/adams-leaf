use std::ops::Range;

type Pair = (Range<u32>, usize);

#[derive(Clone, Debug, Default)]
pub struct IntervalMap {
    inner: Vec<Pair>,
}

impl IntervalMap {
    pub fn new() -> Self {
        IntervalMap::default()
    }
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=&Pair> + 'a {
        self.inner.iter()
    }
    pub fn iter_after<'a>(&'a self, start: u32) -> impl Iterator<Item=&Pair> + 'a {
        match self.inner.binary_search_by_key(&start, |i| i.0.end) {
            Ok(pos)  => self.inner[pos..].iter(),
            Err(pos) => self.inner[pos..].iter(),
        }
    }
    pub fn insert(&mut self, key: Range<u32>, value: usize) {
        // TODO debug_assert is safe?
        debug_assert_ne!(value, usize::MAX);
        debug_assert_eq!(self.check_vacant(key.clone(), value), true);
        match self.inner.binary_search_by_key(&key.start, |i| i.0.start) {
            Ok(_) => unreachable!(),
            Err(pos) if self.pred_connected(pos, &key) == Some(value) => {
                self.inner[pos - 1].0.end = key.end;
            }
            Err(pos) => {
                self.inner.insert(pos, (key, value));
            }
        }
    }
    pub fn remove_value(&mut self, value: usize) {
        self.inner.retain(|i| i.1 != value)
    }
    pub fn clear(&mut self) {
        self.inner.clear();
    }
    pub fn check_vacant(&self, key: Range<u32>, value: usize) -> bool {
        match self.inner.binary_search_by_key(&key.start, |i| i.0.start) {
            Ok(_) => false,
            Err(pos) if self.succ_conflicted(pos, &key).is_some() => false,
            Err(pos) if self.pred_conflicted(pos, &key) == Some(value) => true,
            Err(pos) if self.pred_conflicted(pos, &key).is_some() => false,
            Err(_) => true,
        }
    }
    #[inline]
    fn pred_connected(&self, pos: usize, key: &Range<u32>) -> Option<usize> {
        match pos > 0 && self.inner[pos - 1].0.end >= key.start {
            true => Some(self.inner[pos - 1].1),
            false => None,
        }
    }
    #[inline]
    fn pred_conflicted(&self, pos: usize, key: &Range<u32>) -> Option<usize> {
        match pos > 0 && self.inner[pos - 1].0.end > key.start {
            true => Some(self.inner[pos - 1].1),
            false => None,
        }
    }
    #[inline]
    fn succ_conflicted(&self, pos: usize, key: &Range<u32>) -> Option<usize> {
        let len = self.inner.len();
        match pos < len && key.end > self.inner[pos].0.start {
            true => Some(self.inner[pos].1),
            false => None,
        }
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
