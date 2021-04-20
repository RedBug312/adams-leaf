use crate::MAX_QUEUE;
use crate::component::Solution;
use crate::component::FlowTable;
use crate::utils::stream::TSN;
use std::cmp::{Ordering, max};
use std::ops::Range;
use super::gatectrllist::Entry;

const MTU: u32 = 1522;
const BYTES: u32 = 8;

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
        let frame_len = count_frames(spec);
        let gcl = &solution.allocated_tsns;

        let mut schedule = Schedule::new();
        schedule.windows = vec![vec![std::u32::MAX..std::u32::MAX; frame_len]; route.len()];
        schedule.queue = queue;
        let windows = &mut schedule.windows;
        // let queue = schedule.queue;

        for (r, &edge) in route.iter().enumerate() {
            let port = Entry::Port(edge);
            let queue = Entry::Queue(edge, queue);
            for f in 0..frame_len {
                // let frame_size = MTU * BYTES;
                let frame_size = match f == frame_len - 1 {
                    true  => ((spec.size - 1) % MTU + 1) * BYTES,
                    false => MTU * BYTES,
                };
                let transmit_time = network.duration_on(edge, frame_size).ceil() as u32;

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

                // let start = std::time::Instant::now();
//                 let window = egress..(egress + transmit_time);
// 
//                 if r + 1 < route.len() {
//                     let queue_peek = Entry::Queue(route[r+1], schedule.queue);
//                     egress += gcl.query_later_vacant_once(queue_peek, tsn, window).ok_or(())?;
//                 }
                // let elapsed = start.elapsed().as_nanos();
                // println!("-pass{:?}", elapsed);
                // let start = std::time::Instant::now();

                let window = egress..(egress + transmit_time);
                assert_within_deadline(egress + transmit_time, spec)?;

                egress += gcl.query_later_vacant(port, usize::MAX, window, spec.period)
                    .ok_or(())?;
                assert_within_deadline(egress + transmit_time, spec)?;
                // let elapsed = start.elapsed().as_nanos();
                // println!("--pass{:?}", elapsed);
                // let start = std::time::Instant::now();

                windows[r][f] = egress..(egress + transmit_time);

                if r == 0 { continue; }
                let window = windows[r-1][f].start..windows[r][f].start;
                gcl.check_vacant(queue, tsn, window, spec.period)
                    .then(|| true).ok_or(())?;

                windows[r][f] = egress..(egress + transmit_time);
                // let elapsed = start.elapsed().as_nanos();
                // println!("---pass{:?}", elapsed);

                /* for offset in (0..hyperperiod).step_by(hyperperiod as usize) {
                        let window = (offset + egress)..(offset + egress + transmit_time);
                        println!("{:?}", window);
                        let option = gcl.query_later_vacant_once(port, usize::MAX, window);
                        println!("{:?}", option);
                        if let Some(time) = option {
                            egress += time;
                            assert_within_deadline(egress + transmit_time, spec)?;
                            continue;
                        }
                        if r + 1 < route.len() {
                            let queue_peek = Entry::Queue(route[r+1], schedule.queue);
                            let window = (offset + egress)..(offset + egress + transmit_time);
                            let option = gcl.query_later_vacant_once(queue_peek, tsn, window);
                            if let Some(time) = option {
                                egress += time;
                                assert_within_deadline(egress + transmit_time, spec)?;
                                continue;
                            }
                        }
                        assert_within_deadline(egress + transmit_time, spec)?;
                        // break;
                } */
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

#[inline]
fn count_frames(spec: &TSN) -> usize {
    (spec.size as f64 / MTU as f64).ceil() as usize
}

fn assert_within_deadline(arrival: u32, spec: &TSN) -> Result<u32, ()> {
    let delay = arrival - spec.offset;
    match delay <= spec.deadline {
        true  => Ok(delay),
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
    let windows = schedule.windows;
    for (r, &edge) in route.iter().enumerate() {
        let port = Entry::Port(edge);
        let queue = Entry::Queue(edge, schedule.queue);
        for f in 0..windows[r].len() {
            let window = windows[r][f].clone();
            gcl.insert(port, tsn, window, period);

            if r == 0 { continue; }
            let window = windows[r-1][f].start..windows[r][f].start;
            gcl.insert(queue, tsn, window, period);
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::algorithm::Algorithm;
    use crate::cnc::CNC;
    use crate::network::Network;
    use crate::scheduler::GateCtrlList;
    use crate::utils::yaml;
    use super::*;

    fn setup() -> CNC {
        // TODO use a more straight-forward scenario
        let mut network = Network::new();
        network.add_nodes(6, 0);
        network.add_edges(vec![
            (0, 1, 1000.0), (0, 2, 1000.0), (1, 3, 1000.0), (1, 4, 1000.0),
            (2, 3, 1000.0), (2, 5, 1000.0), (3, 5, 1000.0),
        ]);
        let tsns = vec![
            TSN::new(0, 4, 250, 100, 100, 0),
            TSN::new(0, 5, 750, 150, 150, 0),
            TSN::new(0, 4, 500, 200, 200, 0),
            TSN::new(0, 4, 750, 300, 300, 0),
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
        cnc.solution.allocated_tsns = GateCtrlList::new(&network, 600);
        let result = cnc.scheduler.try_calculate_windows(0, 0, &cnc.solution);
        let windows = result.unwrap().windows;
        assert_eq!(windows, [[0..2], [2..4]]);
        let result = cnc.scheduler.try_calculate_windows(2, 0, &cnc.solution);
        let windows = result.unwrap().windows;
        assert_eq!(windows, [[0..4], [4..8]]);
    }
}
