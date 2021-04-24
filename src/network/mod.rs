mod graph;
mod network;

pub use network::Network;
pub use network::NodeIndex;
pub use network::EdgeIndex;

pub type Path = Vec<EdgeIndex>;

pub const MTU: u32 = 1500;
