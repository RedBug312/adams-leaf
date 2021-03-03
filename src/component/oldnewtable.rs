use crate::component::FlowTable;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OldNew<T: Clone + Eq> {
    Old(T),
    New,
}

pub type OldNewTable<T> = FlowTable<OldNew<T>>;
