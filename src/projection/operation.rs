use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::source_ref::SourceRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionOperationKind {
    Create,
    Update,
    Delete,
    Comment,
    Label,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectionOperation {
    pub operation_id: String,
    pub kind: ProjectionOperationKind,
    pub target: SourceRef,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_operation_round_trips_with_payload() {
        let value = serde_json::json!({
            "operation_id": "calendar-delete-synthetic",
            "kind": "delete",
            "target": {"source_kind": "google_calendar", "source_id": "abcde"},
            "summary": "Synthetic cancelled appointment",
            "payload": {"external_id": "abcde", "summary": "Synthetic cancelled appointment"}
        });
        let operation: ProjectionOperation = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(operation.kind, ProjectionOperationKind::Delete);
        assert_eq!(serde_json::to_value(operation).unwrap(), value);
    }
}
