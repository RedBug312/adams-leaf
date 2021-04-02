use serde::{Serialize, Deserialize};


#[derive(Clone, Serialize, Deserialize)]
pub struct TSN {
    pub src: usize,
    pub dst: usize,
    pub size: usize,
    pub period: u32,
    pub max_delay: u32,
    pub offset: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AVB {
    pub src: usize,
    pub dst: usize,
    pub size: usize,
    pub period: u32,
    pub max_delay: u32,
    pub avb_type: char,
}


impl TSN {
    pub fn new(src: usize, dst: usize, size: usize, period: u32,
               max_delay: u32, offset: u32) -> Self {
        TSN { src, dst, size, period, max_delay, offset }
    }
}

impl AVB {
    pub fn new(src: usize, dst: usize, size: usize, period: u32,
               max_delay: u32, avb_type: char) -> Self {
        AVB { src, dst, size, period, max_delay, avb_type }
    }
}
