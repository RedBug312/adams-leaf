use std::rc::Rc;
use std::collections::BinaryHeap;
use std::time::Instant;
use super::{algorithm::Eval, base::ants::WeightedState};
use rand_chacha::ChaChaRng;
use rand::{Rng, SeedableRng};
use super::Algorithm;
use super::base::ants::ACOJudgeResult;
use crate::{component::flowtable::FlowArena, network::Network, utils::config::Config};
use crate::component::NetworkWrapper;
use crate::component::evaluator::evaluate_avb_latency_for_kth;
use super::base::ants::ACO;
use super::base::yens::YensAlgo;
use crate::MAX_K;

pub struct AdamsAnt {
    pub aco: ACO,
    pub yens: Rc<YensAlgo>,
}
impl AdamsAnt {
    pub fn new(network: &Network) -> Self {
        let yens = YensAlgo::new(&network, MAX_K);
        AdamsAnt {
            aco: ACO::new(0, MAX_K, None),
            yens: Rc::new(yens),
        }
    }
    pub fn get_candidate_count(&self, src: usize, dst: usize) -> usize {
        self.yens.count_shortest_paths(src, dst)
    }
}

impl Algorithm for AdamsAnt {
    fn configure(&mut self, wrapper: &mut NetworkWrapper, arena: &FlowArena, deadline: Instant, evaluate: Eval) {
        self.aco
            .extend_state_len(arena.len());

        let vis = compute_visibility(wrapper, arena, self);
        let cost = evaluate(wrapper);

        let mut best_dist = distance(cost.0);

        let visibility = &vis;

        let mut rng = ChaChaRng::seed_from_u64(42);
        let mut best_state = WeightedState::new(std::f64::MAX, None);
        #[allow(unused_variables)]
        let mut epoch = 0;
        while Instant::now() < deadline {
            epoch += 1;
            // let (should_stop, local_best_state) =
            //     self.aco.do_single_epoch(&visibility, &mut judge_func, &mut rng);

            let mut max_heap: BinaryHeap<WeightedState> = BinaryHeap::new();
            let state_len = self.aco.get_state_len();
            let mut should_stop = false;
            for _ in 0..self.aco.r {
                let mut cur_state = Vec::<usize>::with_capacity(state_len);
                for i in 0..state_len {
                    let next = select_cluster(&visibility[i], &self.aco.pheromone[i], self.aco.k, self.aco.q0, &mut rng);
                    cur_state.push(next);
                    // TODO online pharamon update
                }

                let cost = compute_aco_dist(wrapper, &cur_state, &mut best_dist, &evaluate);
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
            self.aco.evaporate();

            let local_best_state = self.aco.offline_update(max_heap);

            if local_best_state.get_dist() < best_state.get_dist() {
                best_state = local_best_state;
            }
            if should_stop {
                break;
            }
            #[cfg(debug_assertions)]
            println!("pheromone = {:?}", self.aco.pheromone);
        }
        #[cfg(debug_assertions)]
        println!("ACO epoch = {}", epoch);
        best_state.state.expect("找不到最好的解");
    }
    fn prepare(&mut self, wrapper: &mut NetworkWrapper, arena: &FlowArena) {
        for id in arena.inputs() {
            let (src, dst) = arena.ends(id);
            let candidates = self.yens.k_shortest_paths(src, dst);
            wrapper.candidates.push(candidates);
        }
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


fn compute_visibility(wrapper: &NetworkWrapper, arena: &FlowArena, algo: &AdamsAnt) -> Vec<[f64; MAX_K]> {
    let config = Config::get();
    // TODO 好好設計能見度函式！
    // 目前：路徑長的倒數
    let len = algo.aco.get_state_len();
    let mut vis = vec![[0.0; MAX_K]; len];
    for &id in arena.avbs.iter() {
        let flow = arena.avb(id)
            .expect("Failed to obtain AVB spec from TSN stream");
        for kth in 0..algo.get_candidate_count(flow.src, flow.dst) {
            vis[id][kth] = 1.0 / evaluate_avb_latency_for_kth(wrapper, arena, id, kth) as f64;
        }
        if let Some(route_k) = wrapper.get_old_route(id) {
            // 是舊資料流，調高本來路徑的能見度
            vis[id][route_k] *= config.avb_memory;
        }
    }
    for &id in arena.tsns.iter() {
        let flow = arena.tsn(id)
            .expect("Failed to obtain TSN spec from AVB stream");
        for i in 0..algo.get_candidate_count(flow.src, flow.dst) {
            let route = algo.yens.kth_shortest_path(flow.src, flow.dst, i).unwrap();
            vis[id][i] = 1.0 / route.len() as f64;
        }

        if let Some(route_k) = wrapper.get_old_route(id) {
            // 是舊資料流，調高本來路徑的能見度
            vis[id][route_k] *= config.tsn_memory;
        }
    }
    vis
}

/// 本函式不只會計算距離，如果看見最佳解，還會把該解的網路包裝器記錄回 wrapper 參數
fn compute_aco_dist(
    wrapper: &mut NetworkWrapper,
    state: &Vec<usize>,
    best_dist: &mut f64,
    evaluate: &Eval,
) -> (f64, bool) {
    let mut cur_wrapper = wrapper.clone();

    for (id, &kth) in state.iter().enumerate() {
        // NOTE: 若發現和舊的資料一樣，這個 update_info 函式會自動把它忽略掉
        cur_wrapper.flow_table.pick(id, kth);
    }

    let cost = evaluate(&mut cur_wrapper);
    let dist = distance(cost.0);

    if cost.1 {
        // 快速終止！
        *wrapper = cur_wrapper;
        return cost;
    }

    if dist < *best_dist {
        *best_dist = dist;
        // 記錄 FlowTable 及 GCL
        *wrapper = cur_wrapper;
    }
    cost
}

fn distance(cost: f64) -> f64 {
    let base: f64 = 10.0;
    base.powf(cost - 1.0)
}
