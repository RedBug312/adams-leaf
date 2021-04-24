mod graph;
mod network;

pub use network::Edge;
pub use network::EdgeIndex;
pub use network::Network;
pub use network::Node;
pub use network::NodeIndex;

pub type Path = Vec<EdgeIndex>;

pub const BYTES: u32 = 8;
pub const MTU: u32 = 1522;
pub const SFD: u32 = 8;
pub const IPG: u32 = 12;
