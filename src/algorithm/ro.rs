use crate::{MAX_K, cnc::Toolbox, network::Path};
use crate::component::Solution;
use crate::network::Network;
use rand::{Rng, SeedableRng};
use rand::seq::SliceRandom;
use rand_chacha::ChaChaRng;
use std::time::Instant;
use super::base::yens::Yens;
use super::Algorithm;


const ALPHA_PORTION: f64 = 0.5;


pub struct RO {
    yens: Yens,
    seed: u64,
}


impl Algorithm for RO {
    fn candidates(&self, src: usize, dst: usize) -> &Vec<Path> {
        self.yens.k_shortest_paths(src.into(), dst.into())
    }
    /// 在所有 TT 都被排定的狀況下去執行 GRASP 優化
    fn configure(&mut self, solution: &mut Solution, deadline: Instant, toolbox: Toolbox) {
        let flowtable = solution.flowtable();
        // self.grasp(solution, deadline);
        let mut epoch = 0;
        let mut rng = ChaChaRng::seed_from_u64(self.seed);

        let mut global_best = solution.clone();
        let (mut global_best_cost, _stop) = toolbox.evaluate_cost(&mut global_best);

        'outer: while Instant::now() < deadline {
            epoch += 1;
            let mut neighbor = global_best.clone();

            // PHASE 1: randomized greedy algorithm
            for &nth in flowtable.avbs() {
                let (src, dst) = flowtable.ends(nth);
                let candidate_count = self.candidates(src, dst).len();
                // XXX (candidate_cnt as f64 * ALPHA_PORTION).ceil() outperforms
                let alpha = (candidate_count as f64 * ALPHA_PORTION) as usize;
                // XXX (0..candidate_cnt).choose_multiple outperforms
                let set = choose_n_within_k(alpha, candidate_count, &mut rng);
                let kth = set.into_iter()
                    .min_by_key(|&kth| toolbox.evaluate_wcd(nth, kth, &global_best))
                    .unwrap_or(0);
                neighbor.select(nth, kth);
            }
            let (cost, stop) = toolbox.evaluate_cost(&mut neighbor);
            if cost < global_best_cost {
                // XXX global_best = neighbor.clone() outperforms
                global_best_cost = cost;
            }
            #[cfg(debug_assertions)]
            println!("start iteration #{}", epoch);
            if stop {
                global_best = neighbor;
                break 'outer;
            }

            // PHASE 2: local search by hill climbing
            let mut beta = 0;
            // XXX flowtable.avbs().len() outperforms
            while beta < flowtable.len() {
                if Instant::now() > deadline {
                    println!("{:?}", epoch);
                    println!("{:?}", (global_best_cost, stop));
                    break 'outer; // 找到可行解，返回
                }

                let nth = flowtable.avbs().choose(&mut rng).cloned().unwrap();
                let old_kth = global_best.selection(nth).current().unwrap();
                let (src, dst) = flowtable.ends(nth);
                let candidate_count = self.candidates(src, dst).len();
                let kth = (0..candidate_count)
                    .min_by_key(|&kth| toolbox.evaluate_wcd(nth, kth, &global_best))
                    .unwrap_or(0);

                if old_kth == kth {
                    continue;
                }

                // 實際更新下去，並計算成本
                neighbor.select(nth, kth);
                let (cost, stop) = toolbox.evaluate_cost(&mut neighbor);
                if stop {
                    global_best = neighbor;
                    break 'outer;
                }

                if cost < global_best_cost {
                    global_best = neighbor.clone();
                    global_best_cost = cost;
                    beta = 0;
                } else {
                    // 恢復上一動
                    neighbor.select(nth, old_kth);
                    beta += 1;
                }
            }
            println!("{:?}", epoch);
            println!("{:?}", (global_best_cost, stop));
        }
        *solution = global_best;
    }
}

impl RO {
    pub fn new(network: &Network, seed: u64) -> Self {
        let mut yens = Yens::new(&network, MAX_K);
        yens.compute(&network);
        RO { yens, seed }
    }
}

fn choose_n_within_k(n: usize, k: usize, rng: &mut ChaChaRng) -> Vec<usize> {
    let mut vec = Vec::with_capacity(n);
    for i in 0..k {
        let rand = rng.gen();
        let random: usize = rand;
        vec.push((random, i));
    }
    vec.sort();
    vec.into_iter().map(|(_, i)| i).take(n).collect()
}
