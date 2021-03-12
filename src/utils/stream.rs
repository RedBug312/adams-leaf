use serde::{Serialize, Deserialize};


#[derive(Clone, Serialize, Deserialize)]
pub struct TSN {
    pub size: usize,
    pub src: usize,
    pub dst: usize,
    pub period: u32,
    pub max_delay: u32,
    pub offset: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AVB {
    pub size: usize,
    pub src: usize,
    pub dst: usize,
    pub period: u32,
    pub max_delay: u32,
    pub avb_type: char,
}
