use serde::{Deserialize, Serialize};

use crate::time::UbuTimestamp;
use crate::UbuId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionResultStatus {
    Applied,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationResultStatus {
    Applied,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationResult {
    pub operation_id: String,
    pub status: OperationResultStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionResult {
    pub preview_id: UbuId,
    pub applied_at: UbuTimestamp,
    pub status: ProjectionResultStatus,
    pub operation_results: Vec<OperationResult>,
}
