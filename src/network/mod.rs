mod topology;

pub use topology::{EdgeIndex, Network, NodeIndex};

pub type Path = Vec<EdgeIndex>;

pub const MTU: u32 = 1500;
