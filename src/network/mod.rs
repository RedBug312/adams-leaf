mod graph;
mod network;

pub use network::{EdgeIndex, Network, NodeIndex};

pub type Path = Vec<EdgeIndex>;

pub const MTU: u32 = 1500;
