use serde::{Deserialize, Serialize};

use crate::ids::UbuId;
use crate::provenance::Provenance;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveStatus {
    Open,
    Active,
    Satisfied,
    Abandoned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Objective {
    pub id: UbuId,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: ObjectiveStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    pub provenance: Provenance,
}
