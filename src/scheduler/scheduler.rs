use crate::utils::stream::{FlowID, TSNFlow};
use crate::component::{flowtable::*, GCL};
use crate::MAX_QUEUE;

type FT<T> = FlowTable<T>;
type DT<T> = DiffFlowTable<T>;
type Links = Vec<((usize, usize), f64)>;

const MTU: usize = 1500;

/// 一個大小為 size 的資料流要切成幾個封包才夠？
#[inline(always)]
fn get_frame_cnt(size: usize) -> usize {
    if size % MTU == 0 {
        size / MTU
    } else {
        size / MTU + 1
    }
}

use std::cmp::Ordering;
/// 排序的標準：
/// * `deadline` - 時間較緊的要排前面
/// * `period` - 週期短的要排前面
/// * `route length` - 路徑長的要排前面
fn cmp_flow<T: Eq + Clone, TABLE: IFlowTable<INFO = T>, F: Fn(&TSNFlow, &T) -> Links>(
    id1: FlowID,
    id2: FlowID,
    table: &TABLE,
    get_links: F,
) -> Ordering {
    let flow1 = table.get_tsn(id1).unwrap();
    let flow2 = table.get_tsn(id2).unwrap();
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
            let k = table.get_info(flow1.id).unwrap();
            let rlen_1 = get_links(&flow1, k).len();
            let k = table.get_info(flow2.id).unwrap();
            let rlen_2 = get_links(&flow2, k).len();
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
pub fn schedule_online<T: Eq + Clone, F: Fn(&TSNFlow, &T) -> Links>(
    og_table: &mut FT<T>,
    changed_table: &DT<T>,
    gcl: &mut GCL,
    get_links: F,
) -> Result<bool, ()> {
    let result = schedule_fixed_og(changed_table, gcl, |f, t| get_links(f, t));
    og_table.apply_diff(true, changed_table);
    if !result.is_ok() {
        gcl.clear();
        schedule_fixed_og(og_table, gcl, |f, t| get_links(f, t))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// 也可以當作離線排程算法來使用
fn schedule_fixed_og<T: Eq + Clone, TABLE: IFlowTable<INFO = T>, F: Fn(&TSNFlow, &T) -> Links>(
    table: &TABLE,
    gcl: &mut GCL,
    get_links: F,
) -> Result<(), ()> {
    let mut tsn_ids = Vec::<FlowID>::new();
    for (flow, _) in table.iter_tsn() {
        tsn_ids.push(flow.id);
    }
    tsn_ids.sort_by(|&id1, &id2| cmp_flow(id1, id2, table, |f, t| get_links(f, t)));
    for flow_id in tsn_ids.into_iter() {
        let flow = table.get_tsn(flow_id).unwrap();
        let links = get_links(flow, table.get_info(flow_id).unwrap());
        let mut all_offsets: Vec<Vec<u32>> = vec![];
        // NOTE 一個資料流的每個封包，在單一埠口上必需採用同一個佇列
        let mut ro: Vec<u8> = vec![0; links.len()];
        let k = get_frame_cnt(flow.size);
        let mut m = 0;
        while m < k {
            let offsets = calculate_offsets(flow, &all_offsets, &links, &ro, gcl);
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
            gcl.set_queueid(queue_id, link_id, flow_id);
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
                        flow.spec_data.offset
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
    flow: &TSNFlow,
    all_offsets: &Vec<Vec<u32>>,
    links: &Vec<((usize, usize), f64)>,
    ro: &Vec<u8>,
    gcl: &GCL,
) -> Vec<u32> {
    let mut offsets = Vec::<u32>::with_capacity(links.len());
    let hyper_p = gcl.get_hyper_p();
    for i in 0..links.len() {
        let trans_time = (MTU as f64 / links[i].1).ceil() as u32;
        let arrive_time = if i == 0 {
            // 路徑起始
            if all_offsets.len() == 0 {
                // 資料流的第一個封包
                flow.spec_data.offset
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
fn miss_deadline(cur_offset: u32, trans_time: u32, flow: &TSNFlow) -> bool {
    if cur_offset + trans_time >= flow.spec_data.offset + flow.max_delay {
        // 死線爆炸！
        true
    } else {
        false
    }
}

#[cfg(test)]
mod test;
