use std::cmp::Reverse;
use ordered_float::OrderedFloat;
use priority_queue::PriorityQueue;


pub type MyMinHeap<I> = PriorityQueue<I, Priority>;


#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority (Reverse<OrderedFloat<f64>>);

impl From<f64> for Priority {
    fn from(float: f64) -> Self {
        Self (Reverse(OrderedFloat(float)))
    }
}

impl Into<f64> for Priority {
    fn into(self) -> f64 {
        (self.0).0.into_inner()
    }
}

