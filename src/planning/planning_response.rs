use serde::{Deserialize, Serialize};

use crate::planning::diagnostics::ValidationResult;
use crate::planning::plan::Plan;
use crate::time::UbuTimestamp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningResponse {
    pub schema_version: String,
    pub request_id: String,
    pub responded_at: UbuTimestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<Plan>,
    pub validation: ValidationResult,
}
