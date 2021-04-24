use crate::MAX_K;
use super::heap::MyMinHeap;

const R: usize = 60;
const L: usize = 20;
const TAO0: f64 = 25.0;
const RHO: f64 = 0.5; // 蒸發率
const Q0: f64 = 0.0;
const MAX_PH: f64 = 30.0;
const MIN_PH: f64 = 1.0;

pub type State = (Vec<usize>, f64);

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
    pub fn offline_update(&mut self, mut max_heap: MyMinHeap<Vec<usize>>) -> State {
        let best_state = max_heap.pop().unwrap();
        let best = (best_state.0, best_state.1.into());
        self.update_pheromone(&best);
        for _ in 0..self.l - 1 {
            if let Some(state) = max_heap.pop() {
                let state = (state.0, state.1.into());
                self.update_pheromone(&state);
            } else {
                break;
            }
        }
        best
    }
    fn update_pheromone(&mut self, state: &State) {
        let state_len = self.pheromone.len();
        for i in 0..state_len {
            for j in 0..self.k {
                let mut ph = self.pheromone[i][j];
                if state.0[i] == j {
                    ph += 1.0 / state.1;
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
