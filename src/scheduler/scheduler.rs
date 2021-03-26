use std::cmp::Ordering;
use crate::component::flowtable::FlowArena;
use crate::component::NetworkWrapper;
use crate::component::GCL;
use crate::utils::stream::TSN;
use crate::MAX_QUEUE;

type Links = Vec<((usize, usize), f64)>;

const MTU: usize = 1500;


pub struct Scheduler {}


impl Scheduler {
    pub fn new() -> Self {
        Scheduler {}
    }
    pub fn configure(&self, wrapper: &mut NetworkWrapper, arena: &FlowArena) {
        update_avb(wrapper, arena);
        update_tsn(wrapper, arena);
        wrapper.flow_table.confirm();
    }
}

/// 更新 AVB 資料流表與圖上資訊
fn update_avb(wrapper: &mut NetworkWrapper, arena: &FlowArena) {
    let avbs = &arena.avbs;
    let mut updates = Vec::with_capacity(avbs.len());

    updates.extend(wrapper.flow_table.filter_switch(avbs));
    for &id in updates.iter() {
        let prev = wrapper.flow_table.kth_prev(id)
            .expect("Failed to get prev kth with the given id");
        // NOTE: 因為 wrapper.graph 與 wrapper.get_route 是平行所有權
        let graph = unsafe { &mut (*(wrapper as *mut NetworkWrapper)).graph };
        let route = wrapper.get_kth_route(id, prev);
        graph.update_flowid_on_route(false, id, route);
    }

    let avbs = &arena.avbs;
    updates.extend(wrapper.flow_table.filter_pending(avbs));
    for &id in updates.iter() {
        let next = wrapper.flow_table.kth_next(id)
            .expect("Failed to get next kth with the given id");
        // NOTE: 因為 wrapper.graph 與 wrapper.get_route 是平行所有權
        let graph = unsafe { &mut (*(wrapper as *mut NetworkWrapper)).graph };
        let route = wrapper.get_kth_route(id, next);
        graph.update_flowid_on_route(true, id, route);
    }
}
/// 更新 TSN 資料流表與 GCL
fn update_tsn(wrapper: &mut NetworkWrapper, arena: &FlowArena) {
    let tsns = &arena.tsns;
    let mut updates = Vec::with_capacity(tsns.len());

    updates.extend(wrapper.flow_table.filter_switch(tsns));
    for &id in updates.iter() {
        let prev = wrapper.flow_table.kth_prev(id)
            .expect("Failed to get prev kth with the given id");
        let route = wrapper.get_kth_route(id, prev);
        let links = wrapper
            .network
            .get_links_id_bandwidth(route)
            .iter()
            .map(|(ends, _)| *ends)
            .collect();
        wrapper.gcl.delete_flow(&links, id);
    }

    let _wrapper = wrapper as *const NetworkWrapper;

    let closure = |id| {
        // NOTE: 因為 wrapper.flow_table.get 和 wrapper.get_route_func 和 wrapper.graph 與其它部份是平行所有權
        unsafe {
            let kth = (*_wrapper).flow_table.kth_next(id).unwrap();
            let route = (*_wrapper).candidates[id].get(kth).unwrap();
            (*_wrapper).network.get_links_id_bandwidth(route)
        }
    };

    updates.extend(wrapper.flow_table.filter_pending(tsns));
    // FIXME: stream with choice switch(x, x) is scheduled again
    let result = schedule_fixed_og(&arena, &mut wrapper.gcl, &closure, &updates);
    let result = match result {
        Ok(_) => Ok(false),
        Err(_) => {
            wrapper.gcl.clear();
            schedule_fixed_og(&arena, &mut wrapper.gcl, &closure, &arena.tsns)
                .and(Ok(true))
        }
    };

    wrapper.tsn_fail = result.is_err();
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
fn cmp_flow<F: Fn(usize) -> Links>(
    id1: usize,
    id2: usize,
    arena: &FlowArena,
    get_links: &F,
) -> Ordering {
    let flow1 = arena.tsn(id1).unwrap();
    let flow2 = arena.tsn(id2).unwrap();
    if flow1.max_delay < flow2.max_delay {
        Ordering::Less
    } else if flow1.max_delay > flow2.max_delay {
        Ordering::Greater
    } else {
        if flow1.period < flow2.period {
            Ordering::Less
        } else if flow1.period > flow2.period {
            Ordering::Greater
        } else {
            let rlen_1 = get_links(id1).len();
            let rlen_2 = get_links(id2).len();
            if rlen_1 > rlen_2 {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
    }
}

/// 動態計算 TT 資料流的 Gate Control List
/// * `og_table` - 本來的資料流表（排程之後，TT部份會與 changed_table 合併）
/// * `changed_table` - 被改動到的那部份資料流，包含新增與換路徑
/// * `gcl` - 本來的 Gate Control List
/// * 回傳 - Ok(false) 代表沒事發生，Ok(true) 代表發生大洗牌
// pub fn schedule_online<F: Fn(usize) -> Links>(
//     arena: &FlowArena,
//     og_table: &mut FT,
//     changed_table: &DT,
//     gcl: &mut GCL,
//     get_links: &F,
// ) -> Result<bool, ()> {
//     let result = schedule_fixed_og(arena, gcl, get_links, &changed_table.tsn_diff);
//     og_table.apply_diff(true, changed_table);
//     if !result.is_ok() {
//         gcl.clear();
//         schedule_fixed_og(arena, gcl, get_links, &arena.tsns)?;
//         Ok(true)
//     } else {
//         Ok(false)
//     }
// }

/// 也可以當作離線排程算法來使用
pub fn schedule_fixed_og<F: Fn(usize) -> Links>(
    arena: &FlowArena,
    gcl: &mut GCL,
    get_links: &F,
    tsns: &Vec<usize>,
) -> Result<(), ()> {
    let mut tsn_ids = tsns.clone();
    tsn_ids.sort_by(|&id1, &id2| cmp_flow(id1, id2, arena, get_links));
    for flow_id in tsn_ids.into_iter() {
        let flow = arena.tsn(flow_id).unwrap();
        let links = get_links(flow_id);
        let mut all_offsets: Vec<Vec<u32>> = vec![];
        // NOTE 一個資料流的每個封包，在單一埠口上必需採用同一個佇列
        let mut ro: Vec<u8> = vec![0; links.len()];
        let k = get_frame_cnt(flow.size);
        let mut m = 0;
        while m < k {
            let offsets = calculate_offsets(flow_id, arena, &all_offsets, &links, &ro, gcl);
            if offsets.len() == links.len() {
                m += 1;
                all_offsets.push(offsets);
            } else {
                m = 0;
                all_offsets.clear();
                assign_new_queues(&mut ro)?;
            }
        }

        // 把上面算好的結果塞進 GCL
        for i in 0..links.len() {
            let link_id = links[i].0;
            let queue_id = ro[i];
            let trans_time = ((MTU as f64) / links[i].1).ceil() as u32;
            // 考慮 hyper period 中每個狀況
            let p = flow.period as usize;
            for time_shift in (0..gcl.get_hyper_p()).step_by(p) {
                for m in 0..k {
                    // insert gate evt
                    gcl.insert_gate_evt(
                        link_id,
                        flow_id,
                        queue_id,
                        time_shift + all_offsets[m][i],
                        trans_time,
                    );
                    // insert queue evt
                    let queue_evt_start = if i == 0 {
                        flow.offset
                    } else {
                        all_offsets[m][i - 1] // 前一個埠口一開始傳即視為開始佔用
                    };
                    /*println!("===link={} flow={} queue={} {} {}===",
                    link_id, flow_id , queue_id, all_offsets[m][i], queue_evt_start); */
                    let queue_evt_duration = all_offsets[m][i] - queue_evt_start;
                    gcl.insert_queue_evt(
                        link_id,
                        flow_id,
                        queue_id,
                        time_shift + queue_evt_start,
                        queue_evt_duration,
                    );
                }
            }
        }
    }
    Ok(())
}



/// 回傳值為為一個陣列，若其長度小於路徑長，代表排一排爆開
fn calculate_offsets(
    id: usize,
    arena: &FlowArena,
    all_offsets: &Vec<Vec<u32>>,
    links: &Vec<((usize, usize), f64)>,
    ro: &Vec<u8>,
    gcl: &GCL,
) -> Vec<u32> {
    let flow = arena.tsn(id)
        .expect("Failed to obtain TSN spec from AVB stream");
    let mut offsets = Vec::<u32>::with_capacity(links.len());
    let hyper_p = gcl.get_hyper_p();
    for i in 0..links.len() {
        let trans_time = (MTU as f64 / links[i].1).ceil() as u32;
        let arrive_time = if i == 0 {
            // 路徑起始
            if all_offsets.len() == 0 {
                // 資料流的第一個封包
                flow.offset
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
        let p = flow.period as usize;
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
                    if miss_deadline(cur_offset, trans_time, flow) {
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
                        if miss_deadline(cur_offset, trans_time, flow) {
                            return offsets;
                        }
                        continue;
                    }
                }
                if miss_deadline(cur_offset, trans_time, flow) {
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
