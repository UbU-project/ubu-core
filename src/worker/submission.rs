use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::time::UbuTimestamp;
use crate::worker::authority::WorkerAuthority;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerSubmission {
    pub submission_id: String,
    pub submitted_at: UbuTimestamp,
    pub authority: WorkerAuthority,
    pub payload: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerResultStatus {
    Ok,
    Failed,
    NeedsReview,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerResult {
    pub submission_id: String,
    pub completed_at: UbuTimestamp,
    pub status: WorkerResultStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
}
