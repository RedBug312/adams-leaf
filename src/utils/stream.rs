use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct TSN {
    pub src: usize,
    pub dst: usize,
    pub size: u32,
    pub period: u32,
    pub deadline: u32,
    pub offset: u32,
}

#[derive(Deserialize, Clone)]
pub struct AVB {
    pub src: usize,
    pub dst: usize,
    pub size: u32,
    pub period: u32,
    pub deadline: u32,
    pub class: char,
}


impl TSN {
    pub fn new(src: usize, dst: usize, size: u32, period: u32,
               deadline: u32, offset: u32) -> Self {
        TSN { src, dst, size, period, deadline, offset }
    }
}

impl AVB {
    pub fn new(src: usize, dst: usize, size: u32, period: u32,
               deadline: u32, class: char) -> Self {
        AVB { src, dst, size, period, deadline, class }
    }
}
