use crate::scheduler::{Entry, Event};
use std::ops::Range;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("stream #{0:02} reached queue limit")]
    IncrementQueueError(usize),
    #[error("stream #{1:02} queried no vacant for {2:?} per {3} on {0:?}: {4:?}")]
    QueryVacantError(Entry, usize, Range<u32>, u32, Vec<Event>),
    #[error("stream #{1:02} checked no vacant for {2:?} per {3} on {0:?}: {4:?}")]
    CheckVacantError(Entry, usize, Range<u32>, u32, Vec<Event>),
    #[error("stream #{1:02} exceeded deadline {3} for {2:?} on {0:?}, windows {4:?}")]
    ExceedDeadlineError(Entry, usize, Range<u32>, u32, Vec<Vec<Range<u32>>>),
}
