use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::source_ref::SourceRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionOperationKind {
    Create,
    Update,
    Comment,
    Label,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionOperation {
    pub operation_id: String,
    pub kind: ProjectionOperationKind,
    pub target: SourceRef,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}
