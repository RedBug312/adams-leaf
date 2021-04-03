use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Config {
    /// TSN 排程失敗
    pub w0: f64,
    /// AVB 排程失敗的數量
    pub w1: f64,
    /// 重排路徑的成本
    pub w2: f64,
    /// AVB 的平均 Worst case delay
    pub w3: f64,
    /// 快速終止模式，看見第一組可行解即返回
    pub fast_stop: bool,
    /// 計算能見度時，TSN 對舊路徑的偏好程度
    pub tsn_memory: f64,
    /// 計算能見度時，AVB 對舊路徑的偏好程度
    pub avb_memory: f64,
    /// 演算法最多能執行的時間，以微秒計
    pub t_limit: u128,
    /// 執行實驗的次數
    pub exp_times: usize,
}
