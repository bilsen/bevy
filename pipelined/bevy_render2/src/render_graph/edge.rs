use super::NodeId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge {
    input_node: NodeId,
    output_node: NodeId,
}

impl Edge {
    pub fn new(input_node: NodeId, output_node: NodeId) -> Self {
        Edge {
            input_node,
            output_node,
        }
    }
    pub fn get_input_node(&self) -> NodeId {
        self.input_node
    }

    pub fn get_output_node(&self) -> NodeId {
        self.output_node
    }
}
