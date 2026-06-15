use serde::{Deserialize, Serialize};

use crate::planning::diagnostics::ValidationResult;
use crate::planning::plan::Plan;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairResponse {
    pub schema_version: String,
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repaired_plan: Option<Plan>,
    pub validation: ValidationResult,
}
