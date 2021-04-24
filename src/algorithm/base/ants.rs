use crate::MAX_K;
use super::heap::MyMinHeap;

const R: usize = 60;
const L: usize = 20;
const TAO0: f64 = 25.0;
const RHO: f64 = 0.5; // 蒸發率
const Q0: f64 = 0.0;
const MAX_PH: f64 = 30.0;
const MIN_PH: f64 = 1.0;

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
    pub fn new(n: usize, k: usize, tao0: Option<f64>) -> Self {
        assert!(k <= MAX_K, "K值必需在 {} 以下", MAX_K);
        let tao0 = tao0.unwrap_or(TAO0);
        let pheromone = vec![[tao0; MAX_K]; n];
        AntColony { pheromone, k, tao0, r: R, l: L, rho: RHO, q0: Q0, max_ph: MAX_PH, min_ph: MIN_PH }
    }
    pub fn resize_pheromone(&mut self, new_len: usize) {
        let tao0 = self.tao0;
        self.pheromone.resize_with(new_len, || [tao0; MAX_K]);
    }
    pub fn evaporate(&mut self) {
        debug_assert!(self.rho <= 1.0);
        for nth in 0..self.pheromone.len() {
            for kth in 0..self.k {
                let pheromone = (1.0 - self.rho) * self.pheromone[nth][kth];
                self.pheromone[nth][kth] = f64::max(pheromone, self.min_ph);
            }
        }
    }
    pub fn offline_update(&mut self, heap: &MyMinHeap<Vec<usize>>) {
        for (trail, &dist) in heap.iter() {
            self.deposit_pheromone(trail, dist.into());
        }
    }
    fn deposit_pheromone(&mut self, trail: &[usize], dist: f64) {
        debug_assert!(dist.is_sign_positive());
        for nth in 0..self.pheromone.len() {
            let kth = trail[nth];
            let pheromone = self.pheromone[nth][kth] + 1.0 / dist;
            self.pheromone[nth][kth] = f64::min(pheromone, self.max_ph);
        }
    }
}
