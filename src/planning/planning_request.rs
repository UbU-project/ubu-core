use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::authority::AuthoritySource;
use crate::serde_helpers::Duration;
use crate::time::UbuTimestamp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSpec {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimate: Option<Duration>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanningRequest {
    pub request_id: String,
    pub requested_at: UbuTimestamp,
    pub authority_source: AuthoritySource,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_specs: Vec<TaskSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,
}
