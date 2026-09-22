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
#[non_exhaustive]
pub struct Objective {
    pub id: UbuId,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: ObjectiveStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    #[serde(default, skip_serializing_if = "super::ObjectiveMode::is_one_time")]
    pub mode: super::ObjectiveMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence: Option<super::RecurrenceSchedule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routine_instance_template: Option<super::RoutineInstanceTemplate>,
    pub provenance: Provenance,
}

impl Objective {
    pub fn new(id: UbuId, title: String, status: ObjectiveStatus, provenance: Provenance) -> Self {
        Self {
            id,
            title,
            status,
            provenance,
            description: None,
            priority: None,
            mode: super::ObjectiveMode::OneTime,
            recurrence: None,
            routine_instance_template: None,
        }
    }
}
