use crate::utils::config::Config;


#[derive(Clone, Copy, Debug)]
pub struct RoutingCost {
    pub tsn_schedule_fail: bool,
    pub avb_fail_cnt: u32,
    pub avb_wcd: f64,
    pub reroute_overhead: u32,
    pub avb_cnt: usize,
    pub tsn_cnt: usize,
}


impl RoutingCost {
    pub fn objectives(&self) -> [f64; 4] {
        let mut objs = [0f64; 4];
        objs[0] = self.tsn_schedule_fail as u8 as f64;
        objs[1] = self.avb_fail_cnt as f64 / self.avb_cnt as f64;
        objs[2] = self.reroute_overhead as f64 / (self.avb_cnt + self.tsn_cnt) as f64;
        objs[3] = self.avb_wcd / self.avb_cnt as f64;
        objs
    }
    pub fn compute(&self) -> f64 {
        let config = Config::get();
        let cost = self.compute_without_reroute_cost();
        cost + config.w2 * self.reroute_overhead as f64 / (self.avb_cnt + self.tsn_cnt) as f64
    }
    pub fn compute_without_reroute_cost(&self) -> f64 {
        let config = Config::get();
        let mut cost = 0.0;
        if self.tsn_schedule_fail {
            cost += config.w0;
        }
        cost += config.w1 * self.avb_fail_cnt as f64 / self.avb_cnt as f64;
        cost += config.w3 * self.avb_wcd / self.avb_cnt as f64;
        cost
    }
    pub fn show_brief(list: Vec<Self>) {
        let mut all_avb_fail_cnt = 0;
        let mut all_avb_wcd = 0.0;
        let mut all_reroute_cnt = 0;
        let mut all_cost = 0.0;
        let times = list.len() as f64;
        println!(
            "{0: <10} {1: <10} {2: <10} {3: <20} total cost",
            "", "#avb fail", "#reroute", "sum of wcd/deadline"
        );
        for (i, cost) in list.iter().enumerate() {
            if cost.tsn_schedule_fail {
                println!("#{}:\tTSN Schedule Fail!", i);
            } else {
                all_avb_fail_cnt += cost.avb_fail_cnt;
                all_reroute_cnt += cost.reroute_overhead;
                all_avb_wcd += cost.avb_wcd;
                all_cost += cost.compute();
                println!(
                    "{0: <10} {1: <10} {2: <10} {3: <20} {4}",
                    format!("test #{}", i),
                    cost.avb_fail_cnt,
                    cost.reroute_overhead,
                    cost.avb_wcd,
                    cost.compute()
                );
            }
        }
        println!(
            "{0: <10} {1: <10} {2: <10} {3: <20} {4}",
            "average:",
            all_avb_fail_cnt as f64 / times,
            all_reroute_cnt as f64 / times,
            all_avb_wcd / times,
            all_cost / times,
        );
    }
}
