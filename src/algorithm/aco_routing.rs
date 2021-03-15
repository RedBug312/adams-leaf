use super::AdamsAnt;
use crate::utils::config::Config;
use crate::component::{NetworkWrapper, RoutingCost};
use crate::algorithm::aco::ACOJudgeResult;
use crate::MAX_K;
use std::time::Instant;

pub fn do_aco(wrapper: &mut NetworkWrapper, algo: &mut AdamsAnt, time_limit: u128) {
    let time = Instant::now();

    let vis = compute_visibility(wrapper, algo);

    let mut best_dist = dist_computing(&wrapper.compute_all_cost());
    algo.aco
        .do_aco(time_limit - time.elapsed().as_micros(), &vis, |state| {
            let (cost, dist) = compute_aco_dist(wrapper, state, &mut best_dist);
            if cost.avb_fail_cnt == 0 && Config::get().fast_stop {
                // 找到可行解，且為快速終止模式
                ACOJudgeResult::Stop(dist)
            } else {
                ACOJudgeResult::KeepOn(dist)
            }
        });
}

fn compute_visibility(wrapper: &NetworkWrapper, algo: &AdamsAnt) -> Vec<[f64; MAX_K]> {
    let config = Config::get();
    // TODO 好好設計能見度函式！
    // 目前：路徑長的倒數
    let len = algo.aco.get_state_len();
    let mut vis = vec![[0.0; MAX_K]; len];
    for &id in wrapper.get_flow_table().iter_avb() {
        let flow = wrapper.get_flow_table().get_avb(id)
            .expect("Failed to obtain AVB spec with an invalid id");
        for i in 0..algo.get_candidate_count(flow.src, flow.dst) {
            vis[id][i] = 1.0 / wrapper.compute_avb_wcd(id, Some(i)) as f64;
        }
        if let Some(route_k) = wrapper.get_old_route(id) {
            // 是舊資料流，調高本來路徑的能見度
            vis[id][route_k] *= config.avb_memory;
        }
    }
    for &id in wrapper.get_flow_table().iter_tsn() {
        let flow = wrapper.get_flow_table().get_tsn(id)
            .expect("Failed to obtain TSN spec with an invalid id");
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
) -> (RoutingCost, f64) {
    let mut cur_wrapper = wrapper.clone();
    let mut diff = cur_wrapper.get_flow_table().clone_as_diff();

    for (id, &route_k) in state.iter().enumerate() {
        // NOTE: 若發現和舊的資料一樣，這個 update_info 函式會自動把它忽略掉
        diff.update_info_diff(id.into(), route_k);
    }

    cur_wrapper.update_tsn(&diff);
    cur_wrapper.update_avb(&diff);
    let cost = cur_wrapper.compute_all_cost();
    let dist = dist_computing(&cost);

    if Config::get().fast_stop && cost.avb_fail_cnt == 0 {
        // 快速終止！
        *wrapper = cur_wrapper;
        return (cost, dist);
    }

    if dist < *best_dist {
        *best_dist = dist;
        // 記錄 FlowTable 及 GCL
        *wrapper = cur_wrapper;
    }
    (cost, dist)
}

fn dist_computing(cost: &RoutingCost) -> f64 {
    let base: f64 = 10.0;
    base.powf(cost.compute() - 1.0)
}
