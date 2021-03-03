#[derive(Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub struct FlowID(pub(crate) usize);
impl From<usize> for FlowID {
    fn from(i: usize) -> Self {
        FlowID(i)
    }
}
impl Into<usize> for FlowID {
    fn into(self) -> usize {
        self.0
    }
}

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
    pub id: FlowID,
    pub size: usize,
    pub src: usize,
    pub dst: usize,
    pub period: u32,
    pub max_delay: u32,
    pub spec_data: T,
}

pub type TSNFlow = Flow<data::TSNData>;
pub type AVBFlow = Flow<data::AVBData>;

#[derive(Clone, Debug)]
pub enum FlowEnum {
    TSN(TSNFlow),
    AVB(AVBFlow),
}

impl Into<FlowEnum> for Flow<data::TSNData> {
    fn into(self) -> FlowEnum {
        FlowEnum::TSN(self)
    }
}
impl Into<FlowEnum> for Flow<data::AVBData> {
    fn into(self) -> FlowEnum {
        FlowEnum::AVB(self)
    }
}
impl<'a> From<&'a FlowEnum> for &'a Flow<data::TSNData> {
    fn from(flow_enum: &'a FlowEnum) -> &'a TSNFlow {
        if let FlowEnum::TSN(flow) = flow_enum {
            flow
        } else {
            panic!("轉型為 TSN 資料流失敗");
        }
    }
}
impl<'a> From<&'a FlowEnum> for &'a Flow<data::AVBData> {
    fn from(flow_enum: &'a FlowEnum) -> &'a AVBFlow {
        if let FlowEnum::AVB(flow) = flow_enum {
            flow
        } else {
            panic!("轉型為 AVB 資料流失敗");
        }
    }
}
