use crate::{MAX_QUEUE, network::EdgeIndex};
use crate::component::Solution;
use crate::component::FlowTable;
use crate::utils::stream::TSN;
use std::cmp::{Ordering, max};
use std::ops::Range;


const MTU: f64 = 1500.0;


#[derive(Debug, Default)]
struct Schedule {
    windows: Vec<Vec<Range<u32>>>,  // windows[#hop][#frame]
    queue: u8,
}

impl Schedule {
    fn new(route: &Vec<EdgeIndex>, size: usize, queue: u8) -> Self {
        let route_len = route.len();
        let frame_len = (size as f64 / MTU).ceil() as usize;
        static MAX: Range<u32> = std::u32::MAX..std::u32::MAX;
        let windows = vec![vec![MAX.clone(); frame_len]; route_len];
        Schedule { windows, queue }
    }
    fn shape(&self) -> (usize, usize) {
        debug_assert!(0 < self.windows.len());
        (self.windows.len(), self.windows[0].len())
    }
}

#[derive(Default)]
pub struct Scheduler {}


impl Scheduler {
    pub fn new() -> Self {
        Scheduler { ..Default::default() }
    }
    pub fn configure(&self, solution: &mut Solution) {
        self.configure_avbs(solution);
        self.configure_tsns(solution);
        solution.confirm();
    }
    /// 更新 AVB 資料流表與圖上資訊
    fn configure_avbs(&self, solution: &mut Solution) {
        let flowtable = solution.flowtable();
        let avbs = flowtable.avbs();
        let mut targets = Vec::with_capacity(avbs.len());

        targets.extend(flowtable.avbs().iter()
            .filter(|&&avb| solution.selection(avb).is_switch()));
        for &avb in &targets {
            let kth = solution.selection(avb).current().unwrap();
            remove_traversed_avb(solution, avb, kth);
        }

        targets.extend(flowtable.avbs().iter()
            .filter(|&&avb| solution.selection(avb).is_pending()));
        for &avb in &targets {
            let kth = solution.selection(avb).next().unwrap();
            insert_traversed_avb(solution, avb, kth);
        }
    }
    /// 更新 TSN 資料流表與 GCL
    fn configure_tsns(&self, solution: &mut Solution) {
        let flowtable = solution.flowtable();
        let tsns = flowtable.tsns();
        let mut targets = Vec::with_capacity(tsns.len());

        targets.extend(flowtable.tsns().iter()
            .filter(|&&tsn| solution.selection(tsn).is_switch()));
        for &tsn in &targets {
            let kth = solution.selection(tsn).current().unwrap();
            remove_allocated_tsn(solution, tsn, kth);
        }

        targets.extend(flowtable.tsns().iter()
            .filter(|&&tsn| solution.selection(tsn).is_pending()));
        let result = self.try_schedule_tsns(solution, targets);

        if result.is_ok() { return; }

        solution.allocated_tsns.clear();
        targets = tsns.clone();
        self.try_schedule_tsns(solution, targets).unwrap();
    }

    // M. L. Raagaard, P. Pop, M. Gutiérrez and W. Steiner, "Runtime reconfiguration of time-sensitive
    // networking (TSN) schedules for Fog Computing," 2017 IEEE Fog World Congress (FWC), Santa Clara,
    // CA, USA, 2017, pp. 1-6, doi: 10.1109/FWC.2017.8368523.

    fn try_schedule_tsns(&self, solution: &mut Solution, tsns: Vec<usize>)
        -> Result<(), ()> {
        let flowtable = solution.flowtable();
        let mut tsns = tsns;
        tsns.sort_by(|&tsn1, &tsn2|
            compare_tsn(tsn1, tsn2, solution, &flowtable)
        );
        for tsn in tsns {
            let mut queue = 0;
            let kth = solution.selection(tsn).next().unwrap();
            let period = flowtable.tsn_spec(tsn).period;
            loop {
                if let Ok(schedule) = self.try_calculate_windows(tsn, queue, solution) {
                    insert_allocated_tsn(solution, tsn, kth, schedule, period);
                    solution.flag_schedulable(tsn, kth);
                    break;
                }
                if let Err(_) = self.try_increment_queue(&mut queue) {
                    solution.flag_unschedulable(tsn, kth);
                    return Err(());
                }
            }
        }
        Ok(())
    }
    fn try_calculate_windows(&self, tsn: usize, queue: u8,
        solution: &Solution) -> Result<Schedule, ()> {
        let flowtable = solution.flowtable();
        let network = solution.network();
        let spec = flowtable.tsn_spec(tsn);
        let kth = solution.selection(tsn).next().unwrap();
        let route = flowtable.candidate(tsn, kth);
        let gcl = &solution.allocated_tsns;
        let hyperperiod = gcl.hyperperiod();

        let mut schedule = Schedule::new(route, spec.size, queue);
        let (route_len, frame_len) = schedule.shape();
        let windows = &mut schedule.windows;

        for r in 0..route_len {
            let edge = route[r];
            let transmit_time = network.duration_on(edge, MTU).ceil() as u32;
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
                     * 3. 要符合 deadline 的需求
                     */
                    // QUESTION 搞清楚第二點是為什麼？
                    loop {
                        // NOTE 確認沒有其它封包在這個連線上傳輸
                        let option =
                            gcl.get_next_empty_time(edge, time_shift + egress, transmit_time);
                        if let Some(time) = option {
                            egress = time - time_shift;
                            assert_within_deadline(egress + transmit_time, spec)?;
                            continue;
                        }
                        // NOTE 確認傳輸到下個地方時，下個連線的佇列是空的（沒有其它的資料流）
                        if r + 1 < route.len() {
                            // 還不到最後一個節點
                            let option = gcl.get_next_queue_empty_time(
                                route[r + 1],
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
        Ok(schedule)
    }
    fn try_increment_queue(&self, queue: &mut u8) -> Result<u8, u8> {
        *queue += 1;
        match *queue {
            q if q < MAX_QUEUE => Ok(q),
            q => Err(q),
        }
    }
}

/// 排序的標準：
/// * `deadline` - 時間較緊的要排前面
/// * `period` - 週期短的要排前面
/// * `route length` - 路徑長的要排前面
fn compare_tsn(tsn1: usize, tsn2: usize,
    solution: &Solution, flowtable: &FlowTable) -> Ordering {
    let spec1 = flowtable.tsn_spec(tsn1);
    let spec2 = flowtable.tsn_spec(tsn2);
    let routelen = |tsn: usize| {
        let kth = solution.selection(tsn).next().unwrap();
        solution.flowtable().candidate(tsn, kth).len()
    };
    spec1.deadline.cmp(&spec2.deadline)
        .then(spec1.period.cmp(&spec2.period))
        .then(routelen(tsn1).cmp(&routelen(tsn2)).reverse())
}

fn assert_within_deadline(delay: u32, spec: &TSN) -> Result<u32, ()> {
    match delay < spec.offset + spec.deadline {
        true  => Ok(spec.offset + spec.deadline - delay),
        false => Err(()),
    }
}

fn remove_traversed_avb(solution: &mut Solution, avb: usize, kth: usize) {
    let flowtable = solution.flowtable();
    let route = flowtable.candidate(avb, kth);  // kth_route without clone
    for edge in route {
        let set = &mut solution.traversed_avbs[edge.index()];
        set.remove(&avb);
    }
}

fn insert_traversed_avb(solution: &mut Solution, avb: usize, kth: usize) {
    let flowtable = solution.flowtable();
    let route = flowtable.candidate(avb, kth);  // kth_route without clone
    for edge in route {
        let set = &mut solution.traversed_avbs[edge.index()];
        set.insert(avb);
    }
}

fn remove_allocated_tsn(solution: &mut Solution, tsn: usize, kth: usize) {
    let flowtable = solution.flowtable();
    let route = flowtable.candidate(tsn, kth);  // kth_route without clone
    let gcl = &mut solution.allocated_tsns;
    for &edge in route {
        gcl.remove(edge, tsn);
    }
}

fn insert_allocated_tsn(solution: &mut Solution, tsn: usize, kth: usize, schedule: Schedule, period: u32) {
    let flowtable = solution.flowtable();
    let route = flowtable.candidate(tsn, kth);  // kth_route without clone
    let gcl = &mut solution.allocated_tsns;
    let hyperperiod = gcl.hyperperiod();

    let (route_len, frame_len) = schedule.shape();
    let windows = schedule.windows;

    for r in 0..route_len {
        let edge = route[r];
        for f in 0..frame_len {
            for timeshift in (0..hyperperiod).step_by(period as usize) {
                let window = (timeshift + windows[r][f].start)
                    ..(timeshift + windows[r][f].end);
                gcl.insert_gate_evt(edge, tsn, window);
                if r == 0 { continue; }
                let window = (timeshift + windows[r-1][f].start)
                    ..(timeshift + windows[r][f].start);
                gcl.insert_queue_evt(edge, schedule.queue, tsn, window);
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::component::GateCtrlList;
    use crate::algorithm::Algorithm;
    use crate::cnc::CNC;
    use crate::network::Network;
    use crate::utils::yaml;
    use crate::utils::stream::TSN;

    fn setup() -> CNC {
        // TODO use a more straight-forward scenario
        let mut network = Network::new();
        network.add_nodes(6, 0);
        network.add_edges(vec![
            (0, 1, 100.0), (0, 2, 100.0), (1, 3, 100.0), (1, 4, 100.0),
            (2, 3, 100.0), (2, 5, 100.0), (3, 5, 100.0),
        ]);
        let tsns = vec![
            TSN::new(0, 4, 1500, 100, 100, 0),
            TSN::new(0, 5, 4500, 150, 150, 0),
            TSN::new(0, 4, 3000, 200, 200, 0),
            TSN::new(0, 4, 4500, 300, 300, 0),
        ];
        let avbs = vec![];
        let config = yaml::load_config("data/config/default.yaml");
        let mut cnc = CNC::new(network, config);
        cnc.add_streams(tsns, avbs);
        cnc.algorithm.prepare(&mut cnc.solution, &cnc.flowtable);
        cnc
    }

    #[test]
    fn it_calculates_windows() {
        let mut cnc = setup();
        let network = cnc.network;
        cnc.solution.allocated_tsns = GateCtrlList::new(&network, 60);
        let result = cnc.scheduler.try_calculate_windows(0, 0, &cnc.solution);
        let windows = result.unwrap().windows;
        assert_eq!(windows, vec![vec![0..15], vec![15..30]]);
        let result = cnc.scheduler.try_calculate_windows(2, 0, &cnc.solution);
        let windows = result.unwrap().windows;
        assert_eq!(windows, vec![vec![0..15, 15..30], vec![15..30, 30..45]]);
    }
}
