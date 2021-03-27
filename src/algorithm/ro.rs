use crate::MAX_K;
use crate::component::Decision;
use crate::component::evaluator::evaluate_avb_latency_for_kth;
use crate::component::FlowTable;
use crate::network::Network;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::time::Instant;
use super::base::yens::Yens;
use super::Algorithm;
use super::algorithm::Eval;


const ALPHA_PORTION: f64 = 0.5;


pub struct RO {
    yens: Yens,
}


impl Algorithm for RO {
    fn prepare(&mut self, decision: &mut Decision, flowtable: &FlowTable) {
        for id in flowtable.inputs() {
            let (src, dst) = flowtable.ends(id);
            let candidates = self.yens.k_shortest_paths(src, dst);
            decision.candidates.push(candidates);
        }
    }
    /// 在所有 TT 都被排定的狀況下去執行 GRASP 優化
    fn configure(&mut self, decision: &mut Decision, flowtable: &FlowTable, network: &Network, deadline: Instant, evaluate: Eval) {
        // self.grasp(decision, deadline);
        let mut rng = ChaChaRng::seed_from_u64(420);
        let mut iter_times = 0;
        let mut min_cost = evaluate(decision);
        while Instant::now() < deadline {
            iter_times += 1;
            // PHASE 1
            let mut current = decision.clone();
            for &avb in flowtable.avbs() {
                let (src, dst) = flowtable.ends(avb);
                let candidate_cnt = self.get_candidate_count(src, dst);
                let alpha = (candidate_cnt as f64 * ALPHA_PORTION) as usize;
                let set = gen_n_distinct_outof_k(alpha, candidate_cnt, &mut rng);
                let new_route = self.find_min_cost_route(decision, flowtable, network, avb, Some(set));
                current.pick(avb, new_route);
            }
            // PHASE 2
            let cost = evaluate(&mut current);
            if cost.0 < min_cost.0 {
                min_cost = cost;
                // #[cfg(debug_assertions)]
                // println!("found min_cost = {:?} at first glance!", cost);
            }

            #[cfg(debug_assertions)]
            println!("start iteration #{}", iter_times);
            // self.hill_climbing(decision, &mut rng, &deadline, &mut min_cost, current);

            let mut iter_times_inner = 0;
            while Instant::now() < deadline {
                if min_cost.1 {
                    break; // 找到可行解，返回
                }

                let rand = rng
                    .gen_range(0..flowtable.len());
                let target_id = rand.into();
                if flowtable.avb_spec(target_id).is_none() {
                    continue;
                }

                let new_route = self.find_min_cost_route(decision, flowtable, network, target_id, None);
                let old_route = decision
                    .kth(target_id)
                    .unwrap();

                if old_route == new_route {
                    continue;
                }

                // 實際更新下去，並計算成本
                current.pick(target_id, new_route);
                let cost = evaluate(&mut current);

                if cost.0 < min_cost.0 {
                    *decision = current.clone();
                    min_cost = cost.clone();
                    iter_times_inner = 0;

                    // #[cfg(debug_assertions)]
                    // println!("found min_cost = {:?}", cost);
                } else {
                    // 恢復上一動
                    current.pick(target_id, old_route);
                    iter_times_inner += 1;
                    if iter_times_inner == flowtable.len() {
                        //  NOTE: 迭代次數上限與資料流數量掛勾
                        break;
                    }
                }
            }

            if min_cost.1 {
                // 找到可行解，且為快速終止模式
                break;
            }
            println!("{:?}", iter_times);
            println!("{:?}", min_cost);
        }
    }
}

impl RO {
    pub fn new(network: &Network) -> Self {
        let yens = Yens::new(&network, MAX_K);
        RO { yens }
    }
    /// 若有給定候選路徑的子集合，就從中選。若無，則遍歷所有候選路徑
    fn find_min_cost_route(&self, decision: &Decision, flowtable: &FlowTable, network: &Network, id: usize, set: Option<Vec<usize>>) -> usize {
        let (src, dst) = flowtable.ends(id);
        let (mut min_cost, mut best_k) = (std::f64::MAX, 0);
        let mut closure = |k: usize| {
            let cost = evaluate_avb_latency_for_kth(decision, flowtable, network, id, k) as f64;
            if cost < min_cost {
                min_cost = cost;
                best_k = k;
            }
        };
        if let Some(vec) = set {
            for k in vec.into_iter() {
                closure(k);
            }
        } else {
            for k in 0..self.get_candidate_count(src, dst) {
                closure(k);
            }
        }
        best_k
    }
    fn get_candidate_count(&self, src: usize, dst: usize) -> usize {
        self.yens.count_shortest_paths(src, dst)
    }
}

fn gen_n_distinct_outof_k(n: usize, k: usize, rng: &mut ChaChaRng) -> Vec<usize> {
    let mut vec = Vec::with_capacity(n);
    for i in 0..k {
        let rand = rng.gen();
        let random: usize = rand;
        vec.push((random, i));
    }
    vec.sort();
    vec.into_iter().map(|(_, i)| i).take(n).collect()
}
