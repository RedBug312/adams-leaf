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
use super::base::ants::{Ant, AntColony};
use super::base::yens::Yens;

pub struct ACO {
    colony: AntColony,
    yens: Yens,
    memory: Vec<[f64; MAX_K]>,
    seed: u64,
    param: Parameters,
}

impl ACO {
    pub fn new(network: &Network, seed: u64, param: Parameters) -> Self {
        let colony = AntColony::new(0, MAX_K, None);
        let mut yens = Yens::new(network, MAX_K);
        yens.compute(&network);
        let memory = vec![];
        ACO { colony, yens, memory, seed, param }
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
        self.colony.resize_pheromone(flowtable.len());
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
        self.colony.heuristic = self.compute_visibility(solution, &toolbox);
        self.colony.n = solution.flowtable().len();

        #[allow(unused_variables)]
        let mut epoch = 0;
        let mut rng = ChaChaRng::seed_from_u64(self.seed);

        let neighbor = solution.clone();
        let mut global_best = Ant::new(neighbor);
        let (cost, _stop) = toolbox.evaluate_cost(&mut global_best.solution);
        global_best.set_distance_from_cost(cost);

        'outer: while Instant::now() < deadline {
            epoch += 1;
            let mut iteration_best = Ant::empty();
            let mut ants = Vec::with_capacity(self.colony.m);

            for _mth in 0..self.colony.m {
                let neighbor = global_best.solution.clone();
                let mut ant = Ant::new(neighbor);

                for nth in 0..self.colony.n {
                    let kth = select_cluster(&self.colony, nth, &mut rng);
                    ant.solution.select(nth, kth);
                }

                let (cost, stop) = toolbox.evaluate_cost(&mut ant.solution);
                if stop {
                    global_best = ant;
                    break 'outer;
                }

                ant.set_distance_from_cost(cost);
                if ant.distance < iteration_best.distance {
                    iteration_best = ant.clone();
                }
                ants.push(ant);
            }

            self.colony.evaporate();

            for ant in ants {
                self.colony.deposit_pheromone(&ant);
            }
            if iteration_best.distance < global_best.distance {
                global_best = iteration_best;
            }
            #[cfg(debug_assertions)]
            println!("pheromone = {:?}", self.colony.pheromone);
        }

        *solution = global_best.solution;

        #[cfg(debug_assertions)]
        println!("ACO epoch = {}", epoch);
    }
}

fn select_cluster(colony: &AntColony, nth: usize, rng: &mut ChaChaRng) -> usize {
    let k = colony.k;
    let q0 = colony.q0;
    let choices = (0..k)
        .map(|kth| (kth, colony.pheromone[nth][kth] * colony.heuristic[nth][kth]));
    match rng.gen_bool(q0) {
        true  => choices.max_by(|x, y| f64::partial_cmp(&x.1, &y.1).unwrap())
                        .map_or(0, |c| c.0),
        false => choices.collect::<Vec<_>>()
                        .choose_weighted(rng, |item| item.1)
                        .map_or(0, |c| c.0)
    }
}
