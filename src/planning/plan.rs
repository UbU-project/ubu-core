use serde::{Deserialize, Serialize};

use crate::planning::plan_step::PlanStep;
use crate::time::UbuTimestamp;
use crate::UbuId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Candidate,
    Admitted,
    Rejected,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub id: UbuId,
    pub status: PlanStatus,
    pub steps: Vec<PlanStep>,
    pub created_at: UbuTimestamp,
}
