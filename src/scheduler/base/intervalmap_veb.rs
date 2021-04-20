use std::iter::FromIterator;
use std::ops::Range;

#[derive(Clone, Debug, Default)]
pub struct IntervalMap {
    indices: Vec<usize>,
    pairs: Vec<Option<(Range<u32>, usize)>>,
}

impl IntervalMap {
    pub fn new() -> Self {
        let indices = vec![];
        let pairs = vec![None; 1000];
        IntervalMap { indices, pairs }
    }
    pub fn intervals<'a>(&'a self) -> impl Iterator<Item=(Range<u32>, usize)> + 'a {
        self.pairs.iter().cloned()
            .filter_map(|x| x)
    }
    pub fn intervals_after<'a>(&'a self, start: u32) -> impl Iterator<Item=(Range<u32>, usize)> + 'a {
        self.intervals()
            .skip_while(move |p| p.0.end < start)
    }
    pub fn insert(&mut self, key: Range<u32>, value: usize) {
        // TODO debug_assert is safe?
        debug_assert_ne!(value, usize::MAX);
        debug_assert_eq!(self.check_vacant(key.clone(), value), true);
        let index = key.start as usize;
        match self.indices.binary_search(&index) {
            Ok(_) => unreachable!(),
            Err(pos) if self.pred_connected(pos, &key) == Some(value) => {
                let index = self.indices[pos - 1];
                let old = self.pairs[index].as_ref().unwrap();
                self.pairs[index] = Some((old.0.start..key.end, old.1));
            }
            Err(pos) => {
                self.indices.insert(pos, index);
                self.pairs[index] = Some((key, value));
            }
        }
    }
    pub fn remove_value(&mut self, value: usize) {
            // println!("---{:?}", self.indices);
            // println!("---{:?}", Vec::from_iter(self.intervals()));
        let indices = &mut self.indices;
        let pairs = &self.pairs;
        indices.retain(|&i| pairs[i].is_none() || pairs[i].as_ref().unwrap().1 != value);
        let pairs = &mut self.pairs;
        pairs.iter_mut().filter(|p| p.is_some() && p.as_ref().unwrap().1 == value)
            .for_each(|p| *p = None);
            // println!("+++{:?}", self.indices);
            // println!("+++{:?}", Vec::from_iter(self.intervals()));
    }
    pub fn clear(&mut self) {
        self.indices.clear();
        self.pairs = vec![None; 1000];
    }
    pub fn check_vacant(&self, key: Range<u32>, value: usize) -> bool {
        let index = key.start as usize;
        match self.indices.binary_search(&index) {
            Ok(_) => false,
            Err(pos) if self.succ_conflicted(pos, &key).is_some() => false,
            Err(pos) if self.pred_conflicted(pos, &key) == Some(value) => true,
            Err(pos) if self.pred_conflicted(pos, &key).is_some() => false,
            Err(_) => true,
        }
    }
    fn pred_connected(&self, pos: usize, key: &Range<u32>) -> Option<usize> {
        match pos > 0 && self.pairs[self.indices[pos - 1]].as_ref().unwrap().0.end >= key.start {
            true => Some(self.pairs[self.indices[pos - 1]].as_ref().unwrap().1),
            false => None,
        }
    }
    fn pred_conflicted(&self, pos: usize, key: &Range<u32>) -> Option<usize> {
        match pos > 0 && self.pairs[self.indices[pos - 1]].as_ref().unwrap().0.end > key.start {
            true => Some(self.pairs[self.indices[pos - 1]].as_ref().unwrap().1),
            false => None,
        }
    }
    fn succ_conflicted(&self, pos: usize, key: &Range<u32>) -> Option<usize> {
        let len = self.indices.len();
        // println!("{:?}", len);
        // if pos < len {
        //     println!("{:?}", pos);
        //     println!("{:?}", self.indices);
        //     println!("{:?}", Vec::from_iter(self.intervals()));
        //     println!("{:?}", len);
        //     println!("{:?}", self.indices[pos]);
        //     println!("{:?}", self.pairs[self.indices[pos]]);
        // }
        // println!("{:?}", self.pairs.len())};
        match pos < len && key.end > self.pairs[self.indices[pos]].as_ref().unwrap().0.start {
            true => Some(self.pairs[self.indices[pos]].as_ref().unwrap().1),
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
        assert_eq!(map.intervals().collect::<Vec<_>>(), [(2..4, 0), (6..8, 1)]);
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
        let expect = &[(2..4, 0), (4..6, 1), (6..9, 1), (10..12, 1)];
        assert_eq!(map.intervals().collect::<Vec<_>>(), expect);
    }

    #[test]
    fn it_queries_intervals_after() {
        let map = setup();
        assert_eq!(map.intervals_after(0).collect::<Vec<_>>(), &[(2..4, 0), (6..8, 1)]);
        assert_eq!(map.intervals_after(2).collect::<Vec<_>>(), &[(2..4, 0), (6..8, 1)]);
        assert_eq!(map.intervals_after(3).collect::<Vec<_>>(), &[(2..4, 0), (6..8, 1)]);
        assert_eq!(map.intervals_after(4).collect::<Vec<_>>(), &[(2..4, 0), (6..8, 1)]);
        assert_eq!(map.intervals_after(5).collect::<Vec<_>>(), &[(6..8, 1)]);
    }
}
