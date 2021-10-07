use super::NodeId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge {
    before_node: NodeId,
    after_node: NodeId,
}

impl Edge {
    pub fn new(before_node: NodeId, after_node: NodeId) -> Self {
        Edge {
            before_node,
            after_node,
        }
    }
    pub fn get_before_node(&self) -> NodeId {
        self.before_node
    }

    pub fn get_after_node(&self) -> NodeId {
        self.after_node
    }
}
