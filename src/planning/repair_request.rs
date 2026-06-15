use serde::{Deserialize, Serialize};

use crate::planning::diagnostics::SkeletonFailureDiagnostic;
use crate::planning::plan::Plan;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRequest {
    pub schema_version: String,
    pub request_id: String,
    pub plan: Plan,
    pub diagnostics: Vec<SkeletonFailureDiagnostic>,
}
