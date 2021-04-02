use crate::MAX_K;
use std::collections::BinaryHeap;

const R: usize = 60;
const L: usize = 20;
const TAO0: f64 = 25.0;
const RHO: f64 = 0.5; // 蒸發率
const Q0: f64 = 0.0;
const MAX_PH: f64 = 30.0;
const MIN_PH: f64 = 1.0;


pub type State = Vec<usize>;


#[derive(PartialOrd)]
pub struct WeightedState {
    pub neg_dist: f64,
    pub state: Option<State>,
}
impl WeightedState {
    pub fn new(dist: f64, state: Option<State>) -> Self {
        WeightedState {
            neg_dist: -dist,
            state,
        }
    }
    pub fn get_dist(&self) -> f64 {
        -self.neg_dist
    }
}
impl PartialEq for WeightedState {
    fn eq(&self, other: &Self) -> bool {
        return self.neg_dist == other.neg_dist;
    }
}
impl Eq for WeightedState {}
impl Ord for WeightedState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.neg_dist > other.neg_dist {
            std::cmp::Ordering::Greater
        } else if self.neg_dist < other.neg_dist {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Equal
        }
    }
}

pub enum ACOJudgeResult {
    Stop(f64),
    KeepOn(f64),
}

pub struct AntColony {
    pub pheromone: Vec<[f64; MAX_K]>,
    pub k: usize,
    pub r: usize,
    pub l: usize,
    pub rho: f64,
    pub tao0: f64,
    pub q0: f64,
    pub max_ph: f64,
    pub min_ph: f64,
}

impl AntColony {
    pub fn new(state_len: usize, k: usize, tao0: Option<f64>) -> Self {
        assert!(k <= MAX_K, "K值必需在 {} 以下", MAX_K);
        let tao0 = {
            if let Some(t) = tao0 {
                t
            } else {
                TAO0
            }
        };
        AntColony {
            pheromone: (0..state_len).map(|_| [tao0; MAX_K]).collect(),
            tao0,
            k,
            r: R,
            l: L,
            rho: RHO,
            q0: Q0,
            max_ph: MAX_PH,
            min_ph: MIN_PH,
        }
    }
    #[inline(always)]
    pub fn get_state_len(&self) -> usize {
        self.pheromone.len()
    }
    pub fn extend_state_len(&mut self, new_len: usize) {
        if new_len > self.get_state_len() {
            let diff_len = new_len - self.get_state_len();
            let tao0 = self.tao0;
            self.pheromone.extend((0..diff_len).map(|_| [tao0; MAX_K]));
        }
    }
    pub fn evaporate(&mut self) {
        let state_len = self.get_state_len();
        for i in 0..state_len {
            for j in 0..self.k {
                let mut ph = (1.0 - self.rho) * self.pheromone[i][j];
                if ph <= self.min_ph {
                    ph = self.min_ph;
                }
                self.pheromone[i][j] = ph;
            }
        }
    }
    pub fn offline_update(&mut self, mut max_heap: BinaryHeap<WeightedState>) -> WeightedState {
        let best_state = max_heap.pop().unwrap();
        self.update_pheromon(&best_state);
        for _ in 0..self.l - 1 {
            if let Some(w_state) = max_heap.pop() {
                self.update_pheromon(&w_state);
            } else {
                break;
            }
        }
        best_state
    }
    fn update_pheromon(&mut self, w_state: &WeightedState) {
        let dist = w_state.get_dist();
        let state_len = self.pheromone.len();
        for i in 0..state_len {
            for j in 0..self.k {
                let mut ph = self.pheromone[i][j];
                if w_state.state.as_ref().unwrap()[i] == j {
                    ph += 1.0 / dist;
                }
                if ph > self.max_ph {
                    ph = self.max_ph;
                } else if ph < self.min_ph {
                    ph = self.min_ph;
                }
                self.pheromone[i][j] = ph;
            }
        }
    }
}

#[cfg(test)]
mod test {
    // use super::*;
    // #[test]
    // fn test_aco() {
    //     let mut aco = ACO::new(0, 2, None);
    //     aco.extend_state_len(10);
    //     let new_state = aco.do_aco(50000, &vec![[1.0; MAX_K]; 10], |state| {
    //         let mut cost = 6.0;
    //         for (i, &s) in state.iter().enumerate() {
    //             if i % 2 == 0 {
    //                 cost += s as f64;
    //             } else {
    //                 cost -= s as f64;
    //             }
    //         }
    //         ACOJudgeResult::KeepOn(cost / 6.0)
    //     });
    //     assert_eq!(vec![0, 1, 0, 1, 0, 1, 0, 1, 0, 1], new_state);
    // }
}
