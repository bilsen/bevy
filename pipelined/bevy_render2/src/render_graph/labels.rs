use std::borrow::Cow;

use super::{NodeId, RenderGraphId};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NodeLabel {
    Id(NodeId),
    Name(Cow<'static, str>),
}

impl From<&NodeLabel> for NodeLabel {
    fn from(value: &NodeLabel) -> Self {
        value.clone()
    }
}

impl From<String> for NodeLabel {
    fn from(value: String) -> Self {
        NodeLabel::Name(value.into())
    }
}

impl From<&'static str> for NodeLabel {
    fn from(value: &'static str) -> Self {
        NodeLabel::Name(value.into())
    }
}

impl From<NodeId> for NodeLabel {
    fn from(value: NodeId) -> Self {
        NodeLabel::Id(value)
    }
}


#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RenderGraphLabel {
    Id(RenderGraphId),
    Name(Cow<'static, str>),
}

impl From<&RenderGraphLabel> for RenderGraphLabel {
    fn from(value: &RenderGraphLabel) -> Self {
        value.clone()
    }
}

impl From<String> for RenderGraphLabel {
    fn from(value: String) -> Self {
        RenderGraphLabel::Name(value.into())
    }
}

impl From<&'static str> for RenderGraphLabel {
    fn from(value: &'static str) -> Self {
        RenderGraphLabel::Name(value.into())
    }
}

impl From<RenderGraphId> for RenderGraphLabel {
    fn from(value: RenderGraphId) -> Self {
        RenderGraphLabel::Id(value)
    }
}