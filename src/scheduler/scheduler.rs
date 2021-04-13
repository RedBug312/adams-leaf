use crate::MAX_QUEUE;
use crate::component::Solution;
use crate::scheduler::Entry;
use crate::utils::error::Error;
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
    fn new() -> Self {
        Schedule { ..Default::default() }
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
        let _result = self.try_schedule_tsns(solution, targets);
    }

    // M. L. Raagaard, P. Pop, M. Gutiérrez and W. Steiner, "Runtime reconfiguration of time-sensitive
    // networking (TSN) schedules for Fog Computing," 2017 IEEE Fog World Congress (FWC), Santa Clara,
    // CA, USA, 2017, pp. 1-6, doi: 10.1109/FWC.2017.8368523.

    fn try_schedule_tsns(&self, solution: &mut Solution, tsns: Vec<usize>)
        -> Result<(), Error> {
        let flowtable = solution.flowtable();
        let mut tsns = tsns;
        tsns.sort_by(|&tsn1, &tsn2|
            compare_tsn(tsn1, tsn2, solution)
        );
        for tsn in tsns {
            let mut queue = 0;
            let kth = solution.selection(tsn).next().unwrap();
            let period = flowtable.tsn_spec(tsn).unwrap().period;
            loop {
                if let Ok(schedule) = self.try_calculate_windows(tsn, queue, solution)
                    .map_err(|e| println!("{}", e)) {
                    insert_allocated_tsn(solution, tsn, kth, schedule, period);
                    solution.flag_schedulable(tsn, kth);
                    break;
                }
                if let Err(_) = self.try_increment_queue(&mut queue) {
                    solution.flag_unschedulable(tsn, kth);
                    return Err(Error::IncrementQueueError(tsn));
                }
            }
        }
        Ok(())
    }
    // 考慮 hyper period 中每種狀況
    /*
     * 1. 每個連結一個時間只能傳輸一個封包
     * 2. 同個佇列一個時間只能容納一個資料流（但可能容納該資料流的數個封包）
     * 3. 要符合 deadline 的需求
     */
    fn try_calculate_windows(&self, tsn: usize, queue: u8,
        solution: &Solution) -> Result<Schedule, Error> {
        let flowtable = solution.flowtable();
        let network = solution.network();
        let spec = flowtable.tsn_spec(tsn).unwrap();
        let kth = solution.selection(tsn).next().unwrap();
        let route = flowtable.candidate(tsn, kth);
        let frame_len = count_frames(spec);
        let gcl = &solution.allocated_tsns;

        let mut schedule = Schedule::new();
        schedule.windows = vec![vec![0..0; frame_len]; route.len() - 1];
        schedule.queue = queue;
        let windows = &mut schedule.windows;
        // let queue = schedule.queue;

        for (r, ends) in route.windows(2).enumerate() {
            let port = Entry::Port(ends[0], ends[1]);
            let queue = Entry::Queue(ends[0], ends[1], queue);
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

                let window = egress..(egress + transmit_time);

                egress += gcl.query_later_vacant(port, usize::MAX, window.clone(), spec.period)
                    .ok_or_else(|| Error::QueryVacantError(
                        port, tsn, window.clone(), spec.period, gcl.events(port).clone()
                    ))?;
                assert_within_deadline(egress + transmit_time, spec)
                    .ok_or_else(|| Error::ExceedDeadlineError(
                        port, tsn, window.clone(), spec.offset + spec.deadline
                    ))?;

                windows[r][f] = egress..(egress + transmit_time);

                if r == 0 { continue; }
                let window = windows[r-1][f].start..windows[r][f].end;
                gcl.check_vacant(queue, tsn, window.clone(), spec.period).then(|| true)
                    .ok_or_else(|| Error::CheckVacantError(
                        queue, tsn, window, spec.period, gcl.events(queue).clone()
                    ))?;
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
    solution: &Solution) -> Ordering {
    let flowtable = solution.flowtable();
    let spec1 = flowtable.tsn_spec(tsn1).unwrap();
    let spec2 = flowtable.tsn_spec(tsn2).unwrap();
    let routelen = |tsn: usize| {
        let kth = solution.selection(tsn).next().unwrap();
        solution.flowtable().candidate(tsn, kth).len()
    };
    spec1.deadline.cmp(&spec2.deadline)
        .then(spec1.period.cmp(&spec2.period))
        .then(routelen(tsn1).cmp(&routelen(tsn2)).reverse())
}

#[inline]
fn count_frames(spec: &TSN) -> usize {
    (spec.size as f64 / MTU).ceil() as usize
}

fn assert_within_deadline(delay: u32, spec: &TSN) -> Option<u32> {
    match delay < spec.offset + spec.deadline {
        true  => Some(delay),
        false => None,
    }
}

fn remove_traversed_avb(solution: &mut Solution, avb: usize, kth: usize) {
    let flowtable = solution.flowtable();
    let route = flowtable.candidate(avb, kth);  // kth_route without clone
    for ends in route.windows(2) {
        let ends = (ends[0], ends[1]);
        let set = solution.traversed_avbs.get_mut(&ends)
            .expect("Failed to remove traversed avb from an invalid edge");
        set.remove(&avb);
    }
}

fn insert_traversed_avb(solution: &mut Solution, avb: usize, kth: usize) {
    let flowtable = solution.flowtable();
    let route = flowtable.candidate(avb, kth);  // kth_route without clone
    for ends in route.windows(2) {
        let ends = (ends[0], ends[1]);
        let set = solution.traversed_avbs.get_mut(&ends)
            .expect("Failed to insert traversed avb into an invalid edge");
        set.insert(avb);
    }
}

fn remove_allocated_tsn(solution: &mut Solution, tsn: usize, kth: usize) {
    let flowtable = solution.flowtable();
    let route = flowtable.candidate(tsn, kth);  // kth_route without clone
    let gcl = &mut solution.allocated_tsns;
    for ends in route.windows(2) {
        let ends = (ends[0], ends[1]);
        gcl.remove(&ends, tsn);
    }
}

fn insert_allocated_tsn(solution: &mut Solution, tsn: usize, kth: usize, schedule: Schedule, period: u32) {
    let flowtable = solution.flowtable();
    let route = flowtable.candidate(tsn, kth);  // kth_route without clone
    let gcl = &mut solution.allocated_tsns;
    let windows = schedule.windows;
    for (r, ends) in route.windows(2).enumerate() {
        for f in 0..windows[r].len() {
            let port = Entry::Port(ends[0], ends[1]);
            let window = windows[r][f].clone();
            gcl.occupy(port, tsn, window, period);

            if r == 0 { continue; }
            let queue = Entry::Queue(ends[0], ends[1], schedule.queue);
            let window = windows[r-1][f].start..windows[r][f].end;
            gcl.occupy(queue, tsn, window, period);
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::algorithm::Algorithm;
    use crate::cnc::CNC;
    use crate::network::Network;
    use crate::scheduler::GateCtrlList;
    use crate::utils::stream::TSN;
    use crate::utils::yaml;

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
        cnc.solution.allocated_tsns = GateCtrlList::new(60);
        let result = cnc.scheduler.try_calculate_windows(0, 0, &cnc.solution);
        let windows = result.unwrap().windows;
        assert_eq!(windows, vec![vec![0..15], vec![15..30]]);
        let result = cnc.scheduler.try_calculate_windows(2, 0, &cnc.solution);
        let windows = result.unwrap().windows;
        assert_eq!(windows, vec![vec![0..15, 15..30], vec![15..30, 30..45]]);
    }
}
