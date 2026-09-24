use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

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

#[derive(Debug, Clone, PartialEq, Serialize)]
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

#[derive(Deserialize)]
#[serde(remote = "Objective")]
struct ObjectiveWire {
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

impl<'de> Deserialize<'de> for Objective {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let objective = ObjectiveWire::deserialize(deserializer)?;
        objective.validate().map_err(D::Error::custom)?;
        Ok(objective)
    }
}
impl Objective {
    pub fn validate(&self) -> crate::Result<()> {
        if let Some(schedule) = &self.recurrence {
            schedule.validate()?;
        }
        if let Some(template) = &self.routine_instance_template {
            template.validate()?;
        }
        super::validate_objective_routine_fields(
            &self.id,
            self.mode,
            self.recurrence.as_ref(),
            self.routine_instance_template.as_ref(),
            self.priority.is_some(),
        )
    }
}
