use crate::MAX_K;
use crate::component::Solution;

const M: usize = 60;
const L: usize = 20;
const TAO0: f64 = 1.0;
const RHO: f64 = 0.5; // 蒸發率
const Q0: f64 = 0.0;
const MAX_PH: f64 = 1.0;
const MIN_PH: f64 = 0.003;

#[derive(Clone)]
pub struct Ant {
    pub solution: Solution,
    pub distance: f64,
}

impl Ant {
    pub fn new(solution: Solution) -> Self {
        let distance = f64::INFINITY;
        Ant { solution, distance }
    }
    pub fn empty() -> Self {
        let solution = Solution::default();
        let distance = f64::INFINITY;
        Ant { solution, distance }
    }
    pub fn set_distance_from_cost(&mut self, cost: f64) {
        self.distance = cost;
    }
}

pub struct AntColony {
    pub pheromone: Vec<[f64; MAX_K]>,
    pub heuristic: Vec<[f64; MAX_K]>,
    pub n: usize,
    pub m: usize,
    pub k: usize,
    pub l: usize,
    pub rho: f64,
    pub tao0: f64,
    pub q0: f64,
    pub max_ph: f64,
    pub min_ph: f64,
}

impl AntColony {
    pub fn new(n: usize, k: usize, tao0: Option<f64>) -> Self {
        assert!(k <= MAX_K, "K 值必需在 {} 以下", MAX_K);
        let tao0 = tao0.unwrap_or(TAO0);
        let pheromone = vec![[tao0; MAX_K]; n];
        let heuristic = vec![[0.0; MAX_K]; n];
        let n = 0;
        let m = M;
        let l = L;
        let rho = RHO;
        let q0 = Q0;
        let max_ph = MAX_PH;
        let min_ph = MIN_PH;
        AntColony { pheromone, heuristic, n, m, k, l, tao0, rho, q0, max_ph, min_ph }
    }
    pub fn resize_pheromone(&mut self, new_len: usize) {
        let tao0 = self.tao0;
        self.pheromone.resize_with(new_len, || [tao0; MAX_K]);
    }
    pub fn evaporate(&mut self) {
        debug_assert!(self.rho <= 1.0);
        for nth in 0..self.n {
            for kth in 0..self.k {
                let pheromone = (1.0 - self.rho) * self.pheromone[nth][kth];
                self.pheromone[nth][kth] = f64::max(pheromone, self.min_ph);
            }
        }
    }
    pub fn deposit_pheromone(&mut self, ant: &Ant) {
        debug_assert!(ant.distance.is_sign_positive());
        for nth in 0..self.n {
            let kth = ant.solution.selection(nth).current().unwrap();
            let pheromone = self.pheromone[nth][kth] + 1.0 / ant.distance;
            self.pheromone[nth][kth] = f64::min(pheromone, self.max_ph);
        }
    }
}
