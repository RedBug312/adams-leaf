mod graph;
mod streamaware;
mod memorizing;

pub use streamaware::StreamAwareGraph as Network;
pub use memorizing::MemorizingGraph;
pub use graph::Graph;
pub use graph::OnOffGraph;
