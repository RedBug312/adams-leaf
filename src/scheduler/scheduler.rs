use std::cmp::{Ordering, max};
use crate::component::FlowTable;
use crate::component::Decision;
use crate::network::Network;
use crate::utils::stream::TSN;
use crate::MAX_QUEUE;


const MTU: f64 = 1500.0;


#[derive(Default)]
pub struct Schedule {
    pub egress: Vec<Vec<u32>>,  // egress[#hop][#frame]
    pub queue: u8,
}

impl Schedule {
    fn new() -> Self {
        Schedule { ..Default::default() }
    }
}

pub struct Scheduler {}


impl Scheduler {
    pub fn new() -> Self {
        Scheduler {}
    }
    pub fn configure(&self, decision: &mut Decision, flowtable: &FlowTable, network: &Network) {
        update_avb(decision, flowtable);
        update_tsn(decision, flowtable, network);
        decision.confirm();
    }
}

/// 更新 AVB 資料流表與圖上資訊
fn update_avb(decision: &mut Decision, flowtable: &FlowTable) {
    let avbs = flowtable.avbs();
    let mut updates = Vec::with_capacity(avbs.len());

    updates.extend(decision.filter_switch(avbs));
    for &id in updates.iter() {
        let kth = decision.kth(id)
            .expect("Failed to get prev kth with the given id");
        decision.remove_bypassing_avb_on_kth_route(id, kth);
    }

    updates.extend(decision.filter_pending(avbs));
    for &id in updates.iter() {
        let kth = decision.kth_next(id)
            .expect("Failed to get next kth with the given id");
        decision.insert_bypassing_avb_on_kth_route(id, kth);
    }
}

/// 更新 TSN 資料流表與 GCL
fn update_tsn(decision: &mut Decision, flowtable: &FlowTable, network: &Network) {
    let tsns = flowtable.tsns();
    let mut updates = Vec::with_capacity(tsns.len());

    updates.extend(decision.filter_switch(tsns));
    for &id in updates.iter() {
        let prev = decision.kth(id)
            .expect("Failed to get prev kth with the given id");
        let route = decision.kth_route(id, prev);
        let links = network
            .get_links_id_bandwidth(route)
            .iter()
            .map(|(ends, _)| *ends)
            .collect();
        decision.allocated_tsns.delete_flow(&links, id);
    }

    updates.extend(decision.filter_pending(tsns));
    let result = schedule_fixed_og(decision, flowtable, network, updates);
    let result = match result {
        Ok(_) => Ok(false),
        Err(_) => {
            decision.allocated_tsns.clear();
            let updates = tsns.clone();  // reroute all tsns
            schedule_fixed_og(decision, flowtable, network, updates)
                .and(Ok(true))
        }
    };

    decision.tsn_fail = result.is_err();
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

/// 也可以當作離線排程算法來使用
fn schedule_fixed_og(
    decision: &mut Decision,
    flowtable: &FlowTable,
    network: &Network,
    tsns: Vec<usize>,
) -> Result<(), ()> {
    let mut tsns = tsns;
    tsns.sort_by(|&tsn1, &tsn2|
        compare_tsn(tsn1, tsn2, decision, flowtable)
    );
    for tsn in tsns {
        let mut schedule = Schedule::new();
        loop {
            if try_calculate_egress(&mut schedule, tsn, decision, flowtable, network).is_ok() {
                allocate_scheduled_tsn(&schedule, tsn, decision, flowtable, network);
                break;
            }
            if try_increment_queue(&mut schedule).is_err() {
                return Err(());
            }
        }
    }
    Ok(())
}

fn try_increment_queue(schedule: &mut Schedule) -> Result<u8, ()> {
    schedule.queue += 1;
    match schedule.queue {
        q if q < MAX_QUEUE => Ok(q),
        _ => Err(()),
    }
}

fn try_calculate_egress(schedule: &mut Schedule, tsn: usize,
    decision: &Decision, flowtable: &FlowTable, network: &Network) -> Result<u8, ()> {
    let spec = flowtable.tsn_spec(tsn).unwrap();
    let route = decision.route_next(tsn);
    let links = network.get_links_id_bandwidth(route);
    let frame_len = count_frames(spec);
    let gcl = &decision.allocated_tsns;
    let hyperperiod = gcl.hyperperiod();
    let transmit_times = route.windows(2)
        .map(|ends| network.duration_on(ends, MTU))
        .map(|frac| frac.ceil() as u32)
        .collect::<Vec<u32>>();

    schedule.egress = vec![vec![std::u32::MAX; frame_len]; route.len() - 1];
    let egress = &mut schedule.egress;
    let queue = schedule.queue;

    for (r, _ends) in route.windows(2).enumerate() {
        for f in 0..frame_len {
            let transmit_time = transmit_times[r];
            let prev_frame_done = match f {
                0 => spec.offset,
                _ => egress[r][f-1] + transmit_times[r],
            };
            let prev_link_done = match r {
                0 => spec.offset,
                _ => egress[r-1][f] + transmit_times[r-1],
            };
            let arrive_time = max(prev_frame_done, prev_link_done);

            let mut cur_offset = arrive_time;
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
                        gcl.get_next_empty_time(links[r].0, time_shift + cur_offset, transmit_time);
                    if let Some(time) = option {
                        cur_offset = time - time_shift;
                        assert_within_deadline(cur_offset + transmit_time, spec)?;
                        continue;
                    }
                    // NOTE 確認傳輸到下個地方時，下個連線的佇列是空的（沒有其它的資料流）
                    if r < links.len() - 1 {
                        // 還不到最後一個節點
                        let option = gcl.get_next_queue_empty_time(
                            links[r + 1].0,
                            queue,
                            time_shift + (cur_offset + transmit_time),
                        );
                        if let Some(time) = option {
                            cur_offset = time - time_shift;
                            assert_within_deadline(cur_offset + transmit_time, spec)?;
                            continue;
                        }
                    }
                    assert_within_deadline(cur_offset + transmit_time, spec)?;
                    break;
                }
                // QUESTION 是否要檢查 arrive_time ~ cur_offset+trans_time 這段時間中
                // 有沒有發生同個佇列被佔用的事件？
            }
            egress[r][f] = cur_offset;
        }
    }
    Ok(queue)
}

fn allocate_scheduled_tsn(schedule: &Schedule, tsn: usize,
    decision: &mut Decision, flowtable: &FlowTable, network: &Network) {
    // 把上面算好的結果塞進 GCL
    let spec = flowtable.tsn_spec(tsn).unwrap();
    let route = decision.route_next(tsn).clone();
    let frame_len = count_frames(spec);
    let gcl = &mut decision.allocated_tsns;
    let queue = schedule.queue;
    let egress = &schedule.egress;
    let hyperperiod = gcl.hyperperiod();
    let transmit_times = route.windows(2)
        .map(|ends| network.duration_on(ends, MTU))
        .map(|frac| frac.ceil() as u32)
        .collect::<Vec<u32>>();

    for (r, ends) in route.windows(2).enumerate() {
        let ends = (ends[0], ends[1]);
        for f in 0..frame_len {
            // 考慮 hyper period 中每個狀況
            for time_shift in (0..hyperperiod).step_by(spec.period as usize) {
                // insert gate evt
                gcl.insert_gate_evt(
                    ends,
                    tsn,
                    queue,
                    time_shift + egress[r][f],
                    transmit_times[r],
                );
                // insert queue evt
                if r == 0 { continue; }
                let queue_evt_start = egress[r-1][f]; // 前一個埠口一開始傳即視為開始佔用
                let queue_evt_duration = egress[r][f] - queue_evt_start;
                gcl.insert_queue_evt(
                    ends,
                    tsn,
                    queue,
                    time_shift + queue_evt_start,
                    queue_evt_duration,
                );
            }
        }
    }
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
