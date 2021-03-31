use crate::MAX_K;
use crate::component::Decision;
use crate::component::evaluator::evaluate_avb_wcd_for_kth;
use crate::component::FlowTable;
use crate::network::Network;
use crate::utils::config::Config;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::collections::BinaryHeap;
use std::time::Instant;
use super::Algorithm;
use super::algorithm::Eval;
use super::base::ants::AntColony;
use super::base::ants::ACOJudgeResult;
use super::base::ants::WeightedState;
use super::base::yens::Yens;


pub struct ACO {
    ants: AntColony,
    yens: Yens,
    memory: Vec<[f64; MAX_K]>,
    seed: u64,
}


impl ACO {
    pub fn new(network: &Network, seed: u64) -> Self {
        let ants = AntColony::new(0, MAX_K, None);
        let yens = Yens::new(&network, MAX_K);
        let memory = vec![];
        ACO { ants, yens, memory, seed }
    }
    pub fn get_candidate_count(&self, src: usize, dst: usize) -> usize {
        self.yens.count_shortest_paths(src, dst)
    }
}

impl Algorithm for ACO {
    fn prepare(&mut self, decision: &mut Decision, flowtable: &FlowTable) {
        for id in flowtable.inputs() {
            let (src, dst) = flowtable.ends(id);
            let candidates = self.yens.k_shortest_paths(src, dst);
            decision.candidates.push(candidates);
        }
        // before initial scheduler configure
        let config = Config::get();
        self.memory = vec![[1.0; MAX_K]; flowtable.len()];
        for &tsn in flowtable.tsns() {
            if let Some(kth) = decision.kth(tsn) {
                self.memory[tsn][kth] = config.tsn_memory;
            }
        }
        for &avb in flowtable.avbs() {
            if let Some(kth) = decision.kth(avb) {
                self.memory[avb][kth] = config.avb_memory;
            }
        }
    }
    fn configure(&mut self, decision: &mut Decision, flowtable: &FlowTable, network: &Network, deadline: Instant, evaluate: Eval) {
        self.ants.extend_state_len(flowtable.len());

        let vis = compute_visibility(decision, flowtable, network, self);
        let cost = evaluate(decision);

        let mut best_dist = distance(cost.0);

        let visibility = &vis;

        let mut rng = ChaChaRng::seed_from_u64(self.seed);
        let mut best_state = WeightedState::new(std::f64::MAX, None);
        #[allow(unused_variables)]
        let mut epoch = 0;
        while Instant::now() < deadline {
            epoch += 1;
            // let (should_stop, local_best_state) =
            //     self.aco.do_single_epoch(&visibility, &mut judge_func, &mut rng);

            let mut max_heap: BinaryHeap<WeightedState> = BinaryHeap::new();
            let state_len = self.ants.get_state_len();
            let mut should_stop = false;
            for _ in 0..self.ants.r {
                let mut cur_state = Vec::<usize>::with_capacity(state_len);
                for i in 0..state_len {
                    let next = select_cluster(&visibility[i], &self.ants.pheromone[i], self.ants.k, self.ants.q0, &mut rng);
                    cur_state.push(next);
                    // TODO online pharamon update
                }

                let cost = compute_aco_dist(decision, &cur_state, &mut best_dist, &evaluate);
                let dist = distance(cost.0);
                let judge = if cost.1 {
                    // 找到可行解，且為快速終止模式
                    ACOJudgeResult::Stop(dist)
                } else {
                    ACOJudgeResult::KeepOn(dist)
                };

                match judge {
                    ACOJudgeResult::KeepOn(dist) => {
                        max_heap.push(WeightedState::new(dist, Some(cur_state)));
                    }
                    ACOJudgeResult::Stop(dist) => {
                        max_heap.push(WeightedState::new(dist, Some(cur_state)));
                        should_stop = true;
                        break;
                    }
                }
            }
            self.ants.evaporate();

            let local_best_state = self.ants.offline_update(max_heap);

            if local_best_state.get_dist() < best_state.get_dist() {
                best_state = local_best_state;
            }
            if should_stop {
                break;
            }
            #[cfg(debug_assertions)]
            println!("pheromone = {:?}", self.ants.pheromone);
        }
        #[cfg(debug_assertions)]
        println!("ACO epoch = {}", epoch);
        best_state.state.expect("找不到最好的解");
    }
}


fn select_cluster(visibility: &[f64; MAX_K], pheromone: &[f64; MAX_K], k: usize, q0: f64, rng: &mut ChaChaRng) -> usize {
    if rng.gen_range(0.0..1.0) < q0 {
        // 直接選可能性最大者
        let (mut max_i, mut max) = (0, std::f64::MIN);
        for i in 0..k {
            if max < pheromone[i] * visibility[i] {
                max = pheromone[i] * visibility[i];
                max_i = i;
            }
        }
        max_i
    } else {
        // 走隨機過程
        let mut sum = 0.0;
        for i in 0..k {
            sum += pheromone[i] * visibility[i];
        }
        let rand_f = rng.gen_range(0.0..sum);
        let mut accumulation = 0.0;
        for i in 0..k {
            accumulation += pheromone[i] * visibility[i];
            if accumulation >= rand_f {
                return i;
            }
        }
        k - 1
    }
}


fn compute_visibility(decision: &Decision, flowtable: &FlowTable, network: &Network, algo: &ACO) -> Vec<[f64; MAX_K]> {
    // TODO 好好設計能見度函式！
    // 目前：路徑長的倒數
    let len = flowtable.len();
    let mut vis = vec![[0.0; MAX_K]; len];
    for &avb in flowtable.avbs() {
        let (src, dst) = flowtable.ends(avb);
        for kth in 0..algo.get_candidate_count(src, dst) {
            let wcd = evaluate_avb_wcd_for_kth(avb, kth, decision, flowtable, network) as f64;
            vis[avb][kth] = 1.0 / wcd * algo.memory[avb][kth];
        }
    }
    for &tsn in flowtable.tsns() {
        let (src, dst) = flowtable.ends(tsn);
        for kth in 0..algo.get_candidate_count(src, dst) {
            let route = algo.yens.kth_shortest_path(src, dst, kth).unwrap();
            vis[tsn][kth] = 1.0 / route.len() as f64 * algo.memory[tsn][kth];
        }
    }
    vis
}

/// 本函式不只會計算距離，如果看見最佳解，還會把該解的網路包裝器記錄回 decision 參數
fn compute_aco_dist(
    decision: &mut Decision,
    state: &Vec<usize>,
    best_dist: &mut f64,
    evaluate: &Eval,
) -> (f64, bool) {
    let mut current = decision.clone();

    for (id, &kth) in state.iter().enumerate() {
        // NOTE: 若發現和舊的資料一樣，這個 update_info 函式會自動把它忽略掉
        current.pick(id, kth);
    }

    let cost = evaluate(&mut current);
    let dist = distance(cost.0);

    if cost.1 {
        // 快速終止！
        *decision = current;
        return cost;
    }

    if dist < *best_dist {
        *best_dist = dist;
        // 記錄 FlowTable 及 GCL
        *decision = current;
    }
    cost
}

fn distance(cost: f64) -> f64 {
    let base: f64 = 10.0;
    base.powf(cost - 1.0)
}
