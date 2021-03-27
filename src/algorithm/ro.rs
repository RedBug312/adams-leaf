use super::{Algorithm, algorithm::Eval};
use crate::{component::flowtable::FlowArena, network::Network};
use crate::component::NetworkWrapper;
use crate::component::evaluator::evaluate_avb_latency_for_kth;
use super::base::yens::YensAlgo;
use crate::MAX_K;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::rc::Rc;
use std::time::Instant;

const ALPHA_PORTION: f64 = 0.5;


pub struct RO {
    yens: Rc<YensAlgo>,
}


impl Algorithm for RO {
    fn prepare(&mut self, wrapper: &mut NetworkWrapper, arena: &FlowArena) {
        for id in arena.inputs() {
            let (src, dst) = arena.ends(id);
            let candidates = self.yens.k_shortest_paths(src, dst);
            wrapper.candidates.push(candidates);
        }
    }
    /// 在所有 TT 都被排定的狀況下去執行 GRASP 優化
    fn configure(&mut self, wrapper: &mut NetworkWrapper, arena: &FlowArena, network: &Network, deadline: Instant, evaluate: Eval) {
        // self.grasp(wrapper, deadline);
        let mut rng = ChaChaRng::seed_from_u64(420);
        let mut iter_times = 0;
        let mut min_cost = evaluate(wrapper);
        while Instant::now() < deadline {
            iter_times += 1;
            // PHASE 1
            let mut cur_wrapper = wrapper.clone();
            for &avb in arena.avbs() {
                let (src, dst) = arena.ends(avb);
                let candidate_cnt = self.get_candidate_count(src, dst);
                let alpha = (candidate_cnt as f64 * ALPHA_PORTION) as usize;
                let set = gen_n_distinct_outof_k(alpha, candidate_cnt, &mut rng);
                let new_route = self.find_min_cost_route(wrapper, arena, network, avb, Some(set));
                cur_wrapper.flow_table.pick(avb, new_route);
                // cur_wrapper.update_single_avb(id, new_route);
            }
            // PHASE 2
            let cost = evaluate(&mut cur_wrapper);
            if cost.0 < min_cost.0 {
                min_cost = cost;
                // #[cfg(debug_assertions)]
                // println!("found min_cost = {:?} at first glance!", cost);
            }

            #[cfg(debug_assertions)]
            println!("start iteration #{}", iter_times);
            // self.hill_climbing(wrapper, &mut rng, &deadline, &mut min_cost, cur_wrapper);

            let mut iter_times_inner = 0;
            while Instant::now() < deadline {
                if min_cost.1 {
                    break; // 找到可行解，返回
                }

                let rand = rng
                    .gen_range(0..arena.len());
                let target_id = rand.into();
                if arena.avb_spec(target_id).is_none() {
                    continue;
                }

                let new_route = self.find_min_cost_route(wrapper, arena, network, target_id, None);
                let old_route = wrapper
                    .get_flow_table()
                    .kth_prev(target_id)
                    .unwrap();

                if old_route == new_route {
                    continue;
                }

                // 實際更新下去，並計算成本
                cur_wrapper.flow_table.pick(target_id, new_route);
                let cost = evaluate(&mut cur_wrapper);

                if cost.0 < min_cost.0 {
                    *wrapper = cur_wrapper.clone();
                    min_cost = cost.clone();
                    iter_times_inner = 0;

                    // #[cfg(debug_assertions)]
                    // println!("found min_cost = {:?}", cost);
                } else {
                    // 恢復上一動
                    cur_wrapper.flow_table.pick(target_id, old_route);
                    iter_times_inner += 1;
                    if iter_times_inner == arena.len() {
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
        let yens = YensAlgo::new(&network, MAX_K);
        RO {
            yens: Rc::new(yens),
        }
    }
    /// 若有給定候選路徑的子集合，就從中選。若無，則遍歷所有候選路徑
    fn find_min_cost_route(&self, wrapper: &NetworkWrapper, arena: &FlowArena, network: &Network, id: usize, set: Option<Vec<usize>>) -> usize {
        let (src, dst) = arena.ends(id);
        let (mut min_cost, mut best_k) = (std::f64::MAX, 0);
        let mut closure = |k: usize| {
            let cost = evaluate_avb_latency_for_kth(wrapper, arena, network, id, k) as f64;
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
