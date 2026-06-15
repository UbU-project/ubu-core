use serde::{Deserialize, Serialize};

use crate::ids::UbuId;
use crate::provenance::Provenance;
use crate::time::UbuTimestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Active,
    Completed,
    Failed,
    Moot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: UbuId,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective_id: Option<UbuId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<UbuTimestamp>,
    pub provenance: Provenance,
}
