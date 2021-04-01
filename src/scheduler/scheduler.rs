use std::{cmp::{Ordering, max}, rc::{Rc, Weak}};
use std::ops::Range;
use crate::component::FlowTable;
use crate::component::Decision;
use crate::network::Network;
use crate::utils::stream::TSN;
use crate::MAX_QUEUE;


const MTU: f64 = 1500.0;


#[derive(Default)]
struct Schedule {
    pub windows: Vec<Vec<Range<u32>>>,  // windows[#hop][#frame]
    pub queue: u8,
}

impl Schedule {
    fn new() -> Self {
        Schedule { ..Default::default() }
    }
}

#[derive(Default)]
pub struct Scheduler {
    flowtable: Weak<FlowTable>,
    network: Weak<Network>,
}


impl Scheduler {
    pub fn new() -> Self {
        Scheduler { ..Default::default() }
    }
    pub fn flowtable(&self) -> Rc<FlowTable> {
        self.flowtable.upgrade().unwrap()
    }
    pub fn flowtable_mut(&mut self) -> &mut Weak<FlowTable> {
        &mut self.flowtable
    }
    pub fn network(&self) -> Rc<Network> {
        self.network.upgrade().unwrap()
    }
    pub fn network_mut(&mut self) -> &mut Weak<Network> {
        &mut self.network
    }
    pub fn configure(&self, decision: &mut Decision) {
        self.configure_avbs(decision);
        self.configure_tsns(decision);
        decision.confirm();
    }
    /// 更新 AVB 資料流表與圖上資訊
    fn configure_avbs(&self, decision: &mut Decision) {
        let flowtable = self.flowtable();
        let avbs = flowtable.avbs();
        let mut targets = Vec::with_capacity(avbs.len());

        targets.extend(decision.filter_switch(avbs));
        for &avb in targets.iter() {
            let kth = decision.kth(avb).unwrap();
            remove_traversed_avb(decision, avb, kth);
        }

        targets.extend(decision.filter_pending(avbs));
        for &avb in targets.iter() {
            let kth = decision.kth_next(avb).unwrap();
            insert_traversed_avb(decision, avb, kth);
        }
    }
    /// 更新 TSN 資料流表與 GCL
    fn configure_tsns(&self, decision: &mut Decision) {
        let flowtable = self.flowtable();
        let tsns = flowtable.tsns();
        let mut targets = Vec::with_capacity(tsns.len());

        targets.extend(decision.filter_switch(tsns));
        for &tsn in targets.iter() {
            let kth = decision.kth(tsn).unwrap();
            remove_allocated_tsn(decision, tsn, kth);
        }

        targets.extend(decision.filter_pending(tsns));
        let result = self.try_schedule_tsns(decision, targets);
        decision.tsn_fail = result.is_err();

        if !decision.tsn_fail { return; }

        decision.allocated_tsns.clear();
        targets = tsns.clone();
        let result = self.try_schedule_tsns(decision, targets);
        decision.tsn_fail = result.is_err();
    }

    // M. L. Raagaard, P. Pop, M. Gutiérrez and W. Steiner, "Runtime reconfiguration of time-sensitive
    // networking (TSN) schedules for Fog Computing," 2017 IEEE Fog World Congress (FWC), Santa Clara,
    // CA, USA, 2017, pp. 1-6, doi: 10.1109/FWC.2017.8368523.

    fn try_schedule_tsns(&self, decision: &mut Decision, tsns: Vec<usize>)
        -> Result<(), ()> {
        let flowtable = self.flowtable();
        let mut tsns = tsns;
        tsns.sort_by(|&tsn1, &tsn2|
            compare_tsn(tsn1, tsn2, decision, &flowtable)
        );
        for tsn in tsns {
            let mut schedule = Schedule::new();
            let kth = decision.kth_next(tsn).unwrap();
            let period = flowtable.tsn_spec(tsn).unwrap().period;
            loop {
                if self.try_calculate_windows(&mut schedule, tsn, decision).is_ok() {
                    insert_allocated_tsn(decision, tsn, kth, schedule, period);
                    break;
                }
                if self.try_increment_queue(&mut schedule).is_err() {
                    return Err(());
                }
            }
        }
        Ok(())
    }
    fn try_calculate_windows(&self, schedule: &mut Schedule, tsn: usize,
        decision: &Decision) -> Result<u8, ()> {
        let flowtable = self.flowtable();
        let network = self.network();
        let spec = flowtable.tsn_spec(tsn).unwrap();
        let route = decision.route_next(tsn);
        let links = network.get_links_id_bandwidth(route);
        let frame_len = count_frames(spec);
        let gcl = &decision.allocated_tsns;
        let hyperperiod = gcl.hyperperiod();

        schedule.windows = vec![vec![std::u32::MAX..std::u32::MAX; frame_len]; route.len() - 1];
        let windows = &mut schedule.windows;
        let queue = schedule.queue;

        for (r, ends) in route.windows(2).enumerate() {
            let transmit_time = network.duration_on(ends, MTU).ceil() as u32;
            for f in 0..frame_len {
                let prev_frame_done = match f {
                    0 => spec.offset,
                    _ => windows[r][f-1].end,
                };
                let prev_link_done = match r {
                    0 => spec.offset,
                    _ => windows[r-1][f].end,
                };
                let ingress = max(prev_frame_done, prev_link_done);

                let mut egress = ingress;  // ignore bridge processing time
                let p = spec.period as usize;
                for time_shift in (0..hyperperiod).step_by(p) {
                    // 考慮 hyper period 中每種狀況
                    /*
                     * 1. 每個連結一個時間只能傳輸一個封包
                     * 2. 同個佇列一個時間只能容納一個資料流（但可能容納該資料流的數個封包）
                     * 3. 要符合 max_delay 的需求
                     */
                    // QUESTION 搞清楚第二點是為什麼？
                    loop {
                        // NOTE 確認沒有其它封包在這個連線上傳輸
                        let option =
                            gcl.get_next_empty_time(links[r].0, time_shift + egress, transmit_time);
                        if let Some(time) = option {
                            egress = time - time_shift;
                            assert_within_deadline(egress + transmit_time, spec)?;
                            continue;
                        }
                        // NOTE 確認傳輸到下個地方時，下個連線的佇列是空的（沒有其它的資料流）
                        if r < links.len() - 1 {
                            // 還不到最後一個節點
                            let option = gcl.get_next_queue_empty_time(
                                links[r + 1].0,
                                queue,
                                time_shift + (egress + transmit_time),
                            );
                            if let Some(time) = option {
                                egress = time - time_shift;
                                assert_within_deadline(egress + transmit_time, spec)?;
                                continue;
                            }
                        }
                        assert_within_deadline(egress + transmit_time, spec)?;
                        break;
                    }
                    // QUESTION 是否要檢查 arrive_time ~ cur_offset+trans_time 這段時間中
                    // 有沒有發生同個佇列被佔用的事件？
                }
                windows[r][f] = egress..(egress + transmit_time);
            }
        }
        Ok(queue)
    }
    fn try_increment_queue(&self, schedule: &mut Schedule) -> Result<u8, ()> {
        schedule.queue += 1;
        match schedule.queue {
            q if q < MAX_QUEUE => Ok(q),
            _ => Err(()),
        }
    }
}

/// 排序的標準：
/// * `deadline` - 時間較緊的要排前面
/// * `period` - 週期短的要排前面
/// * `route length` - 路徑長的要排前面
fn compare_tsn(tsn1: usize, tsn2: usize,
    decision: &Decision, flowtable: &FlowTable) -> Ordering {
    let spec1 = flowtable.tsn_spec(tsn1).unwrap();
    let spec2 = flowtable.tsn_spec(tsn2).unwrap();
    let routelen = |tsn: usize| {
        decision.route_next(tsn).len()
    };
    spec1.max_delay.cmp(&spec2.max_delay)
        .then(spec1.period.cmp(&spec2.period))
        .then(routelen(tsn1).cmp(&routelen(tsn2)).reverse())
}

#[inline]
fn count_frames(spec: &TSN) -> usize {
    (spec.size as f64 / MTU).ceil() as usize
}

fn assert_within_deadline(delay: u32, spec: &TSN) -> Result<u32, ()> {
    match delay < spec.offset + spec.max_delay {
        true  => Ok(spec.offset + spec.max_delay - delay),
        false => Err(()),
    }
}

fn remove_traversed_avb(decision: &mut Decision, avb: usize, kth: usize) {
    let route = &decision.candidates[avb][kth];  // kth_route without clone
    for ends in route.windows(2) {
        let ends = (ends[0], ends[1]);
        let set = decision.traversed_avbs.get_mut(&ends)
            .expect("Failed to remove traversed avb from an invalid edge");
        set.remove(&avb);
    }
}

fn insert_traversed_avb(decision: &mut Decision, avb: usize, kth: usize) {
    let route = &decision.candidates[avb][kth];  // kth_route without clone
    for ends in route.windows(2) {
        let ends = (ends[0], ends[1]);
        let set = decision.traversed_avbs.get_mut(&ends)
            .expect("Failed to insert traversed avb into an invalid edge");
        set.insert(avb);
    }
}

fn remove_allocated_tsn(decision: &mut Decision, tsn: usize, kth: usize) {
    let gcl = &mut decision.allocated_tsns;
    let route = &decision.candidates[tsn][kth];  // kth_route without clone
    for ends in route.windows(2) {
        let ends = (ends[0], ends[1]);
        gcl.remove(&ends, tsn);
    }
}

fn insert_allocated_tsn(decision: &mut Decision, tsn: usize, kth: usize, schedule: Schedule, period: u32) {
    let gcl = &mut decision.allocated_tsns;
    let hyperperiod = gcl.hyperperiod();
    let windows = schedule.windows;
    let route = &decision.candidates[tsn][kth];  // kth_route without clone
    for (r, ends) in route.windows(2).enumerate() {
        let ends = (ends[0], ends[1]);
        for f in 0..windows[r].len() {
            for timeshift in (0..hyperperiod).step_by(period as usize) {
                let window = (timeshift + windows[r][f].start)
                    ..(timeshift + windows[r][f].end);
                gcl.insert_gate_evt(ends, tsn, window);
                if r == 0 { continue; }
                let window = (timeshift + windows[r-1][f].start)
                    ..(timeshift + windows[r][f].start);
                gcl.insert_queue_evt(ends, schedule.queue, tsn, window);
            }
        }
    }
}
