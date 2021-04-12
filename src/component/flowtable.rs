use crate::algorithm::AlgorithmEnum;
use crate::algorithm::Algorithm;
use crate::network::Path;
use crate::utils::stream::{AVB, TSN};
use std::ops::Range;


enum Either {
    TSN(usize, TSN),
    AVB(usize, AVB),
}

#[derive(Default)]
pub struct FlowTable {
    streams: Vec<Either>,
    tsns: Vec<usize>,
    avbs: Vec<usize>,
    inputs: Range<usize>,
    candidates: Vec<Vec<Path>>,
}


impl FlowTable {
    pub fn new() -> Self {
        FlowTable { ..Default::default() }
    }
    pub fn tsns(&self) -> &Vec<usize> {
        &self.tsns
    }
    pub fn avbs(&self) -> &Vec<usize> {
        &self.avbs
    }
    pub fn inputs(&self) -> Range<usize> {
        self.inputs.clone()
    }
    pub fn len(&self) -> usize {
        self.streams.len()
    }
    pub fn tsn_spec(&self, id: usize) -> Option<&TSN> {
        let either = self.streams.get(id)
            .expect("Failed to obtain TSN spec from an invalid id");
        match either {
            Either::TSN(_, spec) => Some(spec),
            Either::AVB(_, _) => None,
        }
    }
    pub fn avb_spec(&self, id: usize) -> Option<&AVB> {
        let either = self.streams.get(id)
            .expect("Failed to obtain AVB spec from an invalid id");
        match either {
            Either::TSN(_, _) => None,
            Either::AVB(_, spec) => Some(spec),
        }
    }
    pub fn ends(&self, id: usize) -> (usize, usize) {
        let either = self.streams.get(id)
            .expect("Failed to obtain end devices from an invalid id");
        match either {
            Either::TSN(_, tsn) => (tsn.src, tsn.dst),
            Either::AVB(_, avb) => (avb.src, avb.dst),
        }
    }
    pub fn append(&mut self, tsns: Vec<TSN>, avbs: Vec<AVB>) {
        let len = self.streams.len();
        for (idx, tsn) in tsns.into_iter().enumerate() {
            self.tsns.push(len + idx);
            self.streams.push(Either::TSN(len + idx, tsn));
        }
        let len = self.streams.len();
        for (idx, avb) in avbs.into_iter().enumerate() {
            self.avbs.push(len + idx);
            self.streams.push(Either::AVB(len + idx, avb));
        }
        self.inputs = self.inputs.end..self.streams.len();
    }
    pub fn append_candidates(&mut self, algorithm: &AlgorithmEnum) {
        for nth in self.inputs() {
            let (src, dst) = self.ends(nth);
            let candidates = algorithm.candidates(src, dst);
            self.candidates.push(candidates.clone());
        }
    }
    pub fn candidate(&self, nth: usize, kth: usize) -> &Path {
        debug_assert!(nth < self.candidates.len());
        debug_assert!(kth < self.candidates[nth].len());
        &self.candidates[nth][kth]
    }
}


#[cfg(test)]
mod tests {
    use crate::utils::stream::{AVB, TSN};
    use super::FlowTable;

    fn setup() -> FlowTable {
        let mut flowtable = FlowTable::new();
        let tsns = vec![
            TSN::new(4, 2, 200, 10, 100, 20),
        ];
        let avbs = vec![
            AVB::new(0, 1, 100, 10, 200, 'A'),
            AVB::new(0, 2, 100, 10, 200, 'A'),
            AVB::new(0, 3, 100, 10, 200, 'A'),
        ];
        flowtable.append(tsns.clone(), avbs.clone());
        flowtable.append(tsns.clone(), avbs.clone());
        flowtable
    }

    #[test]
    fn it_queries_streams() {
        let flowtable = setup();
        assert_eq!(flowtable.len(), 8);
        assert_eq!(flowtable.tsns(), &vec![0, 4]);
        assert_eq!(flowtable.avbs(), &vec![1, 2, 3, 5, 6, 7]);
        assert_eq!(flowtable.inputs(), 4..8);
    }
}
