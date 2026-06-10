use serde::{Deserialize, Serialize};

use crate::planning::diagnostics::ValidationResult;
use crate::planning::plan::Plan;
use crate::time::UbuTimestamp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanningResponse {
    pub request_id: String,
    pub responded_at: UbuTimestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<Plan>,
    pub validation: ValidationResult,
}
