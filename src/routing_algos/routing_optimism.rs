use super::RoutingAlgo;
use crate::config::Config;
use crate::flow::{AVBFlow, Flow, FlowEnum, FlowID, TSNFlow};
use crate::graph_util::StreamAwareGraph;
use crate::network_wrapper::{NetworkWrapper, RoutingCost};
use crate::recorder::flow_table::prelude::*;
use crate::util::YensAlgo;
use crate::MAX_K;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

const ALPHA_PORTION: f64 = 0.5;

fn get_src_dst(flow: &FlowEnum) -> (usize, usize) {
    match flow {
        FlowEnum::AVB(flow) => (flow.src, flow.dst),
        FlowEnum::TSN(flow) => (flow.src, flow.dst),
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

pub struct RO {
    yens_algo: Rc<RefCell<YensAlgo>>,
    compute_time: u128,
    wrapper: NetworkWrapper<usize>,
}

impl RO {
    pub fn new(g: StreamAwareGraph) -> Self {
        let yens_algo = Rc::new(RefCell::new(YensAlgo::default()));
        let tmp_yens = yens_algo.clone();
        tmp_yens.borrow_mut().compute(&g, MAX_K);
        let wrapper = NetworkWrapper::new(g, move |flow_enum, &k| {
            let (src, dst) = get_src_dst(flow_enum);
            tmp_yens.borrow().kth_shortest_path(src, dst, k).unwrap() as *const Vec<usize>
        });
        RO {
            yens_algo,
            compute_time: 0,
            wrapper,
        }
    }
    /// 在所有 TT 都被排定的狀況下去執行 GRASP 優化
    fn grasp(&mut self, time: Instant) {
        let mut rng = ChaChaRng::seed_from_u64(42);
        let mut iter_times = 0;
        let mut min_cost = self.wrapper.compute_all_cost();
        while time.elapsed().as_micros() < Config::get().t_limit {
            iter_times += 1;
            // PHASE 1
            let mut cur_wrapper = self.wrapper.clone();
            let mut diff = cur_wrapper.get_flow_table().clone_as_diff();
            for (flow, _) in cur_wrapper.get_flow_table().iter_avb() {
                let candidate_cnt = self.get_candidate_count(flow);
                let alpha = (candidate_cnt as f64 * ALPHA_PORTION) as usize;
                let set = gen_n_distinct_outof_k(alpha, candidate_cnt, &mut rng);
                let new_route = self.find_min_cost_route(flow, Some(set));
                diff.update_info(flow.id, new_route);
            }
            cur_wrapper.update_avb(&diff);
            // PHASE 2
            let cost = cur_wrapper.compute_all_cost();
            if cost.compute_without_reroute_cost() < min_cost.compute_without_reroute_cost() {
                min_cost = cost;
                // #[cfg(debug_assertions)]
                // println!("found min_cost = {:?} at first glance!", cost);
            }

            #[cfg(debug_assertions)]
            println!("start iteration #{}", iter_times);
            self.hill_climbing(&mut rng, &time, &mut min_cost, cur_wrapper);
            if min_cost.avb_fail_cnt == 0 && Config::get().fast_stop {
                // 找到可行解，且為快速終止模式
                break;
            }
            println!("{:?}", iter_times);
            println!("{:?}", min_cost);
        }
    }
    /// 若有給定候選路徑的子集合，就從中選。若無，則遍歷所有候選路徑
    fn find_min_cost_route(&self, flow: &AVBFlow, set: Option<Vec<usize>>) -> usize {
        let (mut min_cost, mut best_k) = (std::f64::MAX, 0);
        let mut closure = |k: usize| {
            let cost = self.wrapper.compute_avb_wcd(flow, Some(&k)) as f64;
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
            for k in 0..self.get_candidate_count(flow) {
                closure(k);
            }
        }
        best_k
    }
    fn hill_climbing(
        &mut self,
        rng: &mut ChaChaRng,
        time: &std::time::Instant,
        min_cost: &mut RoutingCost,
        mut cur_wrapper: NetworkWrapper<usize>,
    ) {
        let mut iter_times = 0;
        while time.elapsed().as_micros() < Config::get().t_limit {
            if min_cost.avb_fail_cnt == 0 && Config::get().fast_stop {
                return; // 找到可行解，返回
            }

            let rand = rng
                .gen_range(0..cur_wrapper.get_flow_table().get_flow_cnt());
            let target_id = rand.into();
            let target_flow = {
                // TODO 用更好的機制篩選 avb 資料流
                if let Some(t) = self.wrapper.get_flow_table().get_avb(target_id) {
                    t
                } else {
                    continue;
                }
            };

            let new_route = self.find_min_cost_route(target_flow, None);
            let old_route = *self
                .wrapper
                .get_flow_table()
                .get_info(target_flow.id)
                .unwrap();

            let cost = if old_route == new_route {
                continue;
            } else {
                // 實際更新下去，並計算成本
                cur_wrapper.update_single_avb(target_flow, new_route);
                cur_wrapper.compute_all_cost()
            };
            if cost.compute_without_reroute_cost() < min_cost.compute_without_reroute_cost() {
                self.wrapper = cur_wrapper.clone();
                *min_cost = cost.clone();
                iter_times = 0;

                // #[cfg(debug_assertions)]
                // println!("found min_cost = {:?}", cost);
            } else {
                // 恢復上一動
                cur_wrapper.update_single_avb(target_flow, old_route);
                iter_times += 1;
                if iter_times == cur_wrapper.get_flow_table().get_flow_cnt() {
                    //  NOTE: 迭代次數上限與資料流數量掛勾
                    break;
                }
            }
        }
    }
    fn get_candidate_count<T: Clone>(&self, flow: &Flow<T>) -> usize {
        self.yens_algo.borrow().count_shortest_paths(flow.src, flow.dst)
    }
}
impl RoutingAlgo for RO {
    fn add_flows(&mut self, tsns: Vec<TSNFlow>, avbs: Vec<AVBFlow>) {
        // for flow in tsns.iter() {
        //     self.yens_algo
        //         .borrow_mut()
        //         .compute_routes(flow.src, flow.dst);
        // }
        // for flow in avbs.iter() {
        //     self.yens_algo
        //         .borrow_mut()
        //         .compute_routes(flow.src, flow.dst);
        // }
        let init_time = Instant::now();
        self.wrapper.insert(tsns, avbs, 0);

        self.grasp(init_time);

        self.compute_time = init_time.elapsed().as_micros();
    }
    fn del_flows(&mut self, tsns: Vec<TSNFlow>, avbs: Vec<AVBFlow>) {
        unimplemented!();
    }
    fn get_rerouted_flows(&self) -> &Vec<FlowID> {
        unimplemented!();
    }
    fn get_route(&self, id: FlowID) -> &Vec<usize> {
        self.wrapper.get_route(id)
    }
    fn show_results(&self) {
        println!("TT Flows:");
        for (flow, _) in self.wrapper.get_flow_table().iter_tsn() {
            let route = self.get_route(flow.id);
            println!("flow id = {:?}, route = {:?}", flow.id, route);
        }
        println!("AVB Flows:");
        for (flow, _) in self.wrapper.get_flow_table().iter_avb() {
            let route = self.get_route(flow.id);
            let cost = self.wrapper.compute_single_avb_cost(flow);
            println!(
                "flow id = {:?}, route = {:?} avb wcd / max latency = {:?}, reroute = {}",
                flow.id, route, cost.avb_wcd, cost.reroute_overhead
            );
        }
        let all_cost = self.wrapper.compute_all_cost();
        println!("the cost structure = {:?}", all_cost,);
        println!("{}", all_cost.compute());
    }
    fn get_last_compute_time(&self) -> u128 {
        self.compute_time
    }
    fn get_cost(&self) -> RoutingCost {
        self.wrapper.compute_all_cost()
    }
}
