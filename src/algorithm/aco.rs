use crate::component::FlowTable;
use crate::component::Solution;
use crate::network::Network;
use crate::network::Path;
use crate::{MAX_K, cnc::Toolbox, utils::config::Parameters};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use rand::prelude::SliceRandom;
use std::time::Instant;
use super::Algorithm;
use super::base::ants::AntColony;
use super::base::yens::Yens;
use super::base::heap::MyMinHeap;


pub struct ACO {
    ants: AntColony,
    yens: Yens,
    memory: Vec<[f64; MAX_K]>,
    seed: u64,
    param: Parameters,
}


impl ACO {
    pub fn new(network: &Network, seed: u64, param: Parameters) -> Self {
        let ants = AntColony::new(0, MAX_K, None);
        let mut yens = Yens::new(network, MAX_K);
        yens.compute(&network);
        let memory = vec![];
        ACO { ants, yens, memory, seed, param }
    }
    pub fn get_candidate_count(&self, src: usize, dst: usize) -> usize {
        self.yens.count_shortest_paths(src.into(), dst.into())
    }
    fn compute_visibility(&self, solution: &Solution, toolbox: &Toolbox) -> Vec<[f64; MAX_K]> {
        // TODO 好好設計能見度函式！
        // 目前：路徑長的倒數
        let flowtable = solution.flowtable();
        let len = flowtable.len();
        let mut vis = vec![[0.0; MAX_K]; len];
        for &avb in flowtable.avbs() {
            let (src, dst) = flowtable.ends(avb);
            for kth in 0..self.get_candidate_count(src, dst) {
                let wcd = toolbox.evaluate_wcd(avb, kth, solution) as f64;
                vis[avb][kth] = 1.0 / wcd * self.memory[avb][kth];
            }
        }
        for &tsn in flowtable.tsns() {
            let (src, dst) = flowtable.ends(tsn);
            for kth in 0..self.get_candidate_count(src, dst) {
                let route = self.yens.kth_shortest_path(src.into(), dst.into(), kth).unwrap();
                vis[tsn][kth] = 1.0 / route.len() as f64 * self.memory[tsn][kth];
            }
        }
        vis
    }
}

impl Algorithm for ACO {
    fn candidates(&self, src: usize, dst: usize) -> &Vec<Path> {
        self.yens.k_shortest_paths(src.into(), dst.into())
    }
    fn prepare(&mut self, solution: &mut Solution, flowtable: &FlowTable) {
        // before initial scheduler configure
        self.ants.resize_pheromone(flowtable.len());
        self.memory = vec![[1.0; MAX_K]; flowtable.len()];
        for &tsn in flowtable.tsns() {
            if let Some(kth) = solution.selection(tsn).current() {
                self.memory[tsn][kth] = self.param.tsn_memory;
            }
        }
        for &avb in flowtable.avbs() {
            if let Some(kth) = solution.selection(avb).current() {
                self.memory[avb][kth] = self.param.avb_memory;
            }
        }
    }
    fn configure(&mut self, solution: &mut Solution, deadline: Instant, toolbox: Toolbox) {
        let vis = self.compute_visibility(solution, &toolbox);
        let cost = toolbox.evaluate_cost(solution);

        let mut best_dist = distance(cost.0);

        let visibility = &vis;

        let mut rng = ChaChaRng::seed_from_u64(self.seed);
        let state_len = solution.flowtable().len();
        let mut trail = Vec::<usize>::with_capacity(state_len);
        for i in 0..state_len {
            let next = solution.selection(i).current().unwrap();
            trail.push(next);
        }
        #[allow(unused_variables)]
        let mut epoch = 0;
        while Instant::now() < deadline {
            epoch += 1;
            // let (should_stop, local_best_state) =
            //     self.aco.do_single_epoch(&visibility, &mut judge_func, &mut rng);
            let mut heap = MyMinHeap::new();
            let mut should_stop = false;

            for _ in 0..self.ants.r {
                let mut trail = Vec::<usize>::with_capacity(state_len);
                let mut neighbor = solution.clone();

                for nth in 0..state_len {
                    let kth = select_cluster(&visibility[nth], &self.ants.pheromone[nth], self.ants.k, self.ants.q0, &mut rng);
                    trail.push(kth);
                    neighbor.select(nth, kth);
                }
                let (cost, stop) = toolbox.evaluate_cost(&mut neighbor);

                let dist = distance(cost);
                if stop || dist < best_dist {
                    best_dist = dist;
                    *solution = neighbor;
                }
                heap.push(trail, dist.into());

                if stop {
                    should_stop = true;
                    break;
                }
            }
            self.ants.evaporate();
            self.ants.offline_update(&heap);
            if should_stop {
                break;
            }
            #[cfg(debug_assertions)]
            println!("pheromone = {:?}", self.ants.pheromone);
        }
        #[cfg(debug_assertions)]
        println!("ACO epoch = {}", epoch);
    }
}

fn select_cluster(visibility: &[f64; MAX_K], pheromone: &[f64; MAX_K], k: usize, q0: f64, rng: &mut ChaChaRng) -> usize {
    let choices = (0..k)
        .map(|kth| (kth, pheromone[kth] * visibility[kth]));
    match rng.gen_bool(q0) {
        true  => choices.max_by(|x, y| f64::partial_cmp(&x.1, &y.1).unwrap())
                        .map_or(k - 1, |c| c.0),
        false => choices.collect::<Vec<_>>()
                        .choose_weighted(rng, |item| item.1)
                        .map_or(k - 1, |c| c.0)
    }
}

#[inline]
fn distance(cost: f64) -> f64 {
    f64::powf(10.0, cost - 1.0)
}
