use serde::{Deserialize, Serialize};

use crate::policy_summary::PolicySummary;
use crate::projection::operation::ProjectionOperation;
use crate::time::UbuTimestamp;
use crate::UbuId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionPreview {
    pub id: UbuId,
    pub created_at: UbuTimestamp,
    pub operations: Vec<ProjectionOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_summary: Option<PolicySummary>,
}
