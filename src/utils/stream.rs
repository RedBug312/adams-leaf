pub mod data {
    #[derive(Clone, Copy, Debug)]
    pub enum AVBClass {
        A,
        B,
    }
    impl AVBClass {
        pub fn is_class_a(&self) -> bool {
            if let AVBClass::A = self {
                true
            } else {
                false
            }
        }
        pub fn is_class_b(&self) -> bool {
            if let AVBClass::B = self {
                true
            } else {
                false
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct AVBData {
        pub avb_class: AVBClass,
    }

    #[derive(Clone, Debug)]
    pub struct TSNData {
        pub offset: u32,
    }
}

#[derive(Clone, Debug)]
pub struct Flow<T: Clone> {
    pub id: usize,
    pub size: usize,
    pub src: usize,
    pub dst: usize,
    pub period: u32,
    pub max_delay: u32,
    pub spec_data: T,
}

pub type TSNFlow = Flow<data::TSNData>;
pub type AVBFlow = Flow<data::AVBData>;
