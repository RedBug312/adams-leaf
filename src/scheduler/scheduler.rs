use std::cmp::Ordering;
use crate::component::FlowTable;
use crate::component::Decision;
use crate::component::GateCtrlList;
use crate::network::Network;
use crate::utils::stream::TSN;
use crate::MAX_QUEUE;


const MTU: usize = 1500;

#[derive(Default)]
pub struct Schedule {
    pub egress: Vec<Vec<u32>>,  // egress[#hop][#frame]
    pub queue: u8,
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
    // FIXME: stream with choice switch(x, x) is scheduled again
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

/// 一個大小為 size 的資料流要切成幾個封包才夠？
#[inline(always)]
fn get_frame_cnt(size: usize) -> usize {
    if size % MTU == 0 {
        size / MTU
    } else {
        size / MTU + 1
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

/// 動態計算 TT 資料流的 Gate Control List
/// * `og_table` - 本來的資料流表（排程之後，TT部份會與 changed_table 合併）
/// * `changed_table` - 被改動到的那部份資料流，包含新增與換路徑
/// * `gcl` - 本來的 Gate Control List
/// * 回傳 - Ok(false) 代表沒事發生，Ok(true) 代表發生大洗牌
// pub fn schedule_online<F: Fn(usize) -> Links>(
//     flowtable: &FlowTable,
//     og_table: &mut FT,
//     changed_table: &DT,
//     gcl: &mut GCL,
//     get_links: &F,
// ) -> Result<bool, ()> {
//     let result = schedule_fixed_og(flowtable, gcl, get_links, &changed_table.tsn_diff);
//     og_table.apply_diff(true, changed_table);
//     if !result.is_ok() {
//         gcl.clear();
//         schedule_fixed_og(flowtable, gcl, get_links, &flowtable.tsns)?;
//         Ok(true)
//     } else {
//         Ok(false)
//     }
// }

/// 也可以當作離線排程算法來使用
pub fn schedule_fixed_og(
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
        let spec = flowtable.tsn_spec(tsn).unwrap();
        let route = decision.route_next(tsn);
        let links = network.get_links_id_bandwidth(route);
        // NOTE 一個資料流的每個封包，在單一埠口上必需採用同一個佇列
        let frame_count = get_frame_cnt(spec.size);
        let mut all_offsets: Vec<Vec<u32>> = vec![];
        let mut ro: Vec<u8> = vec![0; links.len()];
        let mut m = 0;
        let gcl = &mut decision.allocated_tsns;
        while m < frame_count {
            let offsets = calculate_offsets(tsn, flowtable, &all_offsets, &links, &ro, gcl);
            if offsets.len() == links.len() {
                m += 1;
                all_offsets.push(offsets);
            } else {
                m = 0;
                all_offsets.clear();
                assign_new_queues(&mut ro)?;
            }
        }
        let schedule = Schedule {
            egress: all_offsets,
            queue: ro[0],
        };
        populate_into_gcl(decision, tsn, &schedule, flowtable, network);
    }
    Ok(())
}

fn populate_into_gcl(decision: &mut Decision, tsn: usize, schedule: &Schedule,
    flowtable: &FlowTable, network: &Network) {
    // 把上面算好的結果塞進 GCL
    let spec = flowtable.tsn_spec(tsn).unwrap();
    let route = decision.route_next(tsn).clone();
    let frame_count = get_frame_cnt(spec.size);
    let gcl = &mut decision.allocated_tsns;
    let queue = schedule.queue;
    let egress = &schedule.egress;

    for (r, ends) in route.windows(2).enumerate() {
        let link_id = (ends[0], ends[1]);
        let hyper_period = gcl.get_hyper_p();
        let transmit_time = network.duration_on(ends, MTU as f64).ceil() as u32;
        // 考慮 hyper period 中每個狀況
        for time_shift in (0..hyper_period).step_by(spec.period as usize) {
            for f in 0..frame_count {
                // insert gate evt
                gcl.insert_gate_evt(
                    link_id,
                    tsn,
                    queue,
                    time_shift + egress[f][r],
                    transmit_time,
                );
                // insert queue evt
                if r == 0 { continue; }
                let queue_evt_start = egress[f][r - 1]; // 前一個埠口一開始傳即視為開始佔用
                let queue_evt_duration = egress[f][r] - queue_evt_start;
                gcl.insert_queue_evt(
                    link_id,
                    tsn,
                    queue,
                    time_shift + queue_evt_start,
                    queue_evt_duration,
                );
            }
        }
    }
}


/// 回傳值為為一個陣列，若其長度小於路徑長，代表排一排爆開
fn calculate_offsets(
    tsn: usize,
    flowtable: &FlowTable,
    all_offsets: &Vec<Vec<u32>>,
    links: &Vec<((usize, usize), f64)>,
    ro: &Vec<u8>,
    gcl: &GateCtrlList,
) -> Vec<u32> {
    let spec = flowtable.tsn_spec(tsn)
        .expect("Failed to obtain TSN spec from AVB stream");
    let mut offsets = Vec::<u32>::with_capacity(links.len());
    let hyper_p = gcl.get_hyper_p();
    for i in 0..links.len() {
        let trans_time = (MTU as f64 / links[i].1).ceil() as u32;
        let arrive_time = if i == 0 {
            // 路徑起始
            if all_offsets.len() == 0 {
                // 資料流的第一個封包
                spec.offset
            } else {
                // #m-1 封包完整送出，且經過處理時間
                all_offsets[all_offsets.len() - 1][i] + trans_time
            }
        } else {
            // #m 封包送達，且經過處理時間
            let a = offsets[i - 1] + (MTU as f64 / links[i - 1].1).ceil() as u32;
            if all_offsets.len() == 0 {
                a
            } else {
                // #m-1 封包完整送出，且經過處理時間
                let b = all_offsets[all_offsets.len() - 1][i] + trans_time;
                if a > b {
                    a
                } else {
                    b
                }
            }
        };
        let mut cur_offset = arrive_time;
        let p = spec.period as usize;
        for time_shift in (0..hyper_p).step_by(p) {
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
                    gcl.get_next_empty_time(links[i].0, time_shift + cur_offset, trans_time);
                if let Some(time) = option {
                    cur_offset = time - time_shift;
                    if miss_deadline(cur_offset, trans_time, spec) {
                        return offsets;
                    }
                    continue;
                }
                // NOTE 確認傳輸到下個地方時，下個連線的佇列是空的（沒有其它的資料流）
                if i < links.len() - 1 {
                    // 還不到最後一個節點
                    let option = gcl.get_next_queue_empty_time(
                        links[i + 1].0,
                        ro[i],
                        time_shift + (cur_offset + trans_time),
                    );
                    if let Some(time) = option {
                        cur_offset = time - time_shift;
                        if miss_deadline(cur_offset, trans_time, spec) {
                            return offsets;
                        }
                        continue;
                    }
                }
                if miss_deadline(cur_offset, trans_time, spec) {
                    return offsets;
                }
                break;
            }
            // QUESTION 是否要檢查 arrive_time ~ cur_offset+trans_time 這段時間中有沒有發生同個佇列被佔用的事件？
        }
        offsets.push(cur_offset);
    }
    offsets
}

fn assign_new_queues(ro: &mut Vec<u8>) -> Result<(), ()> {
    // TODO 好好實作這個函式（目前一個資料流只安排個佇列，但在不同埠口上應該可以安排給不同佇列）
    if ro[0] == MAX_QUEUE - 1 {
        Err(())
    } else {
        for i in 0..ro.len() {
            ro[i] += 1;
        }
        Ok(())
    }
}

#[inline(always)]
fn miss_deadline(cur_offset: u32, trans_time: u32, flow: &TSN) -> bool {
    if cur_offset + trans_time >= flow.offset + flow.max_delay {
        // 死線爆炸！
        true
    } else {
        false
    }
}
