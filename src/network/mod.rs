mod graph;
mod network;

pub use network::Edge;
pub use network::EdgeIndex;
pub use network::Network;
pub use network::Node;
pub use network::NodeIndex;

pub type Path = Vec<EdgeIndex>;

pub const MTU: u32 = 250;  // FIXME
pub const BYTES: u32 = 8;
