use crate::utils::stream::FlowID;
use std::collections::HashMap;

use crate::MAX_QUEUE;

fn gcd(a: u32, b: u32) -> u32 {
    if a < b {
        gcd(b, a)
    } else if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}
fn lcm(a: u32, b: u32) -> u32 {
    let g = gcd(a, b);
    (a / g) * b
}

#[cfg(test)]
mod test {
    use super::lcm;
    #[test]
    fn test_lcm() {
        assert_eq!(36, lcm(4, 9));
        assert_eq!(81, lcm(27, 81));
        assert_eq!(84, lcm(12, 21));
    }
}

#[derive(Clone, Debug)]
pub struct GCL {
    hyper_p: u32,
    // TODO 這個資料結構有優化的空間
    gate_evt: Vec<Vec<(u32, u32, u8, FlowID)>>,
    queue_occupy_evt: Vec<[Vec<(u32, u32, FlowID)>; MAX_QUEUE as usize]>,
    queue_map: HashMap<(usize, FlowID), u8>,
    gate_evt_lookup: Vec<Option<Vec<(u32, u32)>>>,
}
impl GCL {
    pub fn new(hyper_p: u32, edge_count: usize) -> Self {
        GCL {
            gate_evt: vec![vec![]; edge_count],
            gate_evt_lookup: vec![None; edge_count],
            queue_occupy_evt: vec![Default::default(); edge_count],
            queue_map: HashMap::new(),
            hyper_p,
        }
    }
    pub fn update_hyper_p(&mut self, new_p: u32) {
        self.hyper_p = lcm(self.hyper_p, new_p);
    }
    pub fn clear(&mut self) {
        let edge_cnt = self.gate_evt.len();
        self.gate_evt = vec![vec![]; edge_cnt];
        self.gate_evt_lookup = vec![None; edge_cnt];
        self.queue_occupy_evt = vec![Default::default(); edge_cnt];
        self.queue_map = HashMap::new();
    }
    pub fn get_hyper_p(&self) -> u32 {
        self.hyper_p
    }
    /// 回傳 `link_id` 上所有閘門關閉事件。
    /// * `回傳值` - 一個陣列，其內容為 (事件開始時間, 事件持續時間);
    pub fn get_gate_events(&self, link_id: usize) -> &Vec<(u32, u32)> {
        assert!(self.gate_evt.len() > link_id, "GCL: 指定了超出範圍的邊");
        if self.gate_evt_lookup[link_id].is_none() {
            // 生成快速查找表
            let mut lookup = Vec::<(u32, u32)>::new();
            let len = self.gate_evt[link_id].len();
            if len > 0 {
                let first_evt = self.gate_evt[link_id][0];
                let mut cur_evt = (first_evt.0, first_evt.1);
                for &(start, duration, ..) in self.gate_evt[link_id][1..len].iter() {
                    if cur_evt.0 + cur_evt.1 == start {
                        // 首尾相接
                        cur_evt.1 += duration; // 把閘門事件延長
                    } else {
                        lookup.push(cur_evt);
                        cur_evt = (start, duration);
                    }
                }
                lookup.push(cur_evt);
            }
            unsafe {
                // NOTE 內部可變，因為這只是加速用的
                let _self = self as *const Self as *mut Self;
                (*_self).gate_evt_lookup[link_id] = Some(lookup);
            }
        }
        self.gate_evt_lookup[link_id].as_ref().unwrap()
    }
    pub fn insert_gate_evt(
        &mut self,
        link_id: usize,
        flow_id: FlowID,
        queue_id: u8,
        start_time: u32,
        duration: u32,
    ) {
        self.gate_evt_lookup[link_id] = None;
        let entry = (start_time, duration, queue_id, flow_id);
        let evts = &mut self.gate_evt[link_id];
        match evts.binary_search(&entry) {
            Ok(_) => panic!("插入重複的閘門事件: link={}, {:?}", link_id, entry),
            Err(pos) => {
                if pos > 0 && evts[pos - 1].0 + evts[pos - 1].1 > start_time {
                    // 開始時間位於前一個事件中
                    panic!(
                        "插入重疊的閘門事件： link={}, {:?} v.s. {:?}",
                        link_id,
                        evts[pos - 1],
                        entry
                    );
                } else {
                    evts.insert(pos, entry)
                }
            }
        }
    }
    pub fn insert_queue_evt(
        &mut self,
        link_id: usize,
        flow_id: FlowID,
        queue_id: u8,
        start_time: u32,
        duration: u32,
    ) {
        if duration == 0 {
            return;
        }
        let entry = (start_time, duration, flow_id);
        let evts = &mut self.queue_occupy_evt[link_id][queue_id as usize];
        match evts.binary_search(&entry) {
            // FIXME: 這個異常有機率發生，試著重現看看！
            Ok(_) => panic!(
                "插入重複的佇列事件: link={}, queue={}, {:?}",
                link_id, queue_id, entry
            ),
            Err(pos) => {
                if pos > 0 && evts[pos - 1].0 + evts[pos - 1].1 >= start_time {
                    // 開始時間位於前一個事件中，則延伸前一個事件
                    evts[pos - 1].1 = start_time + duration - evts[pos - 1].0;
                } else {
                    evts.insert(pos, entry)
                }
            }
        }
    }
    /// 會先確認 start~(start+duration) 這段時間中有沒有與其它事件重疊
    ///
    /// 若否，則回傳 None，應可直接塞進去。若有重疊，則會告知下一個空的時間（但不一定塞得進去）
    pub fn get_next_empty_time(&self, link_id: usize, start: u32, duration: u32) -> Option<u32> {
        assert!(
            self.gate_evt.len() > link_id,
            "GCL: 指定了超出範圍的邊: {}/{}",
            link_id,
            self.gate_evt.len()
        );
        let s1 = self.get_next_spot(link_id, start);
        let s2 = self.get_next_spot(link_id, start + duration);
        if s1.0 != s2.0 {
            // 是不同的閘門事
            Some(s1.0)
        } else if s1.1 {
            // 是同一個閘門事件的開始
            None
        } else {
            // 是同一個閘門事件的結束，代表 start~duration 這段時間正處於該事件之中，重疊了!
            Some(s2.0)
        }
    }
    /// 計算最近的下一個「時間點」，此處的時間點有可能是閘門事件的開啟或結束。
    ///
    /// 回傳一組資料(usize, bool)，前者代表時間，後者代表該時間是閘門事件的開始還是結束（真代表開始）
    fn get_next_spot(&self, link_id: usize, time: u32) -> (u32, bool) {
        // TODO 應該用二元搜索來優化?
        for &(start, duration, ..) in self.gate_evt[link_id].iter() {
            if start > time {
                return (start, true);
            } else if start + duration > time {
                return (start + duration, false);
            }
        }
        (self.hyper_p, true)
    }
    pub fn get_queueid(&self, link_id: usize, flow_id: FlowID) -> u8 {
        *self.queue_map.get(&(link_id, flow_id)).unwrap()
    }
    pub fn set_queueid(&mut self, queueid: u8, link_id: usize, flow_id: FlowID) {
        self.queue_map.insert((link_id, flow_id), queueid);
    }
    /// 回傳 None 者，代表當前即是空的
    pub fn get_next_queue_empty_time(
        &self,
        link_id: usize,
        queue_id: u8,
        time: u32,
    ) -> Option<u32> {
        let evts = &self.queue_occupy_evt[link_id][queue_id as usize];
        for &(start, duration, _) in evts.iter() {
            if start <= time {
                if start + duration > time {
                    return Some(start + duration);
                } else {
                    return None;
                }
            }
        }
        None
    }
    pub fn delete_flow(&mut self, links: &Vec<usize>, flow_id: FlowID) {
        for &link_id in links.iter() {
            self.gate_evt_lookup[link_id] = None;
            let gate_evt = &mut self.gate_evt[link_id];
            let mut i = 0;
            self.queue_map.remove(&(link_id, flow_id));
            while i < gate_evt.len() {
                if gate_evt[i].3 == flow_id {
                    gate_evt.remove(i);
                } else {
                    i += 1;
                }
            }
            for queue_id in 0..MAX_QUEUE {
                let queue_evt = &mut self.queue_occupy_evt[link_id][queue_id as usize];
                let mut i = 0;
                while i < queue_evt.len() {
                    if queue_evt[i].2 == flow_id {
                        queue_evt.remove(i);
                    } else {
                        i += 1;
                    }
                }
            }
        }
    }
}
