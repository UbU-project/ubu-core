use std::collections::BTreeSet;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};

use crate::core::universe_state::{UniverseMutation, UniversePrecondition};
use crate::ids::UbuId;
use crate::provenance::Provenance;
use crate::time::UbuTimestamp;
use crate::{ObjectType, UbuError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Active,
    Completed,
    Failed,
    Moot,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Moot => "moot",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MootReasonCode {
    ExternallySatisfied,
    Superseded,
    Delegated,
    NoLongerRelevant,
    InvalidatedByUniverseChange,
    ReplacedByNewPlanStructure,
    UserDeclaredMoot,
    AutomationObsolete,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Task {
    pub id: UbuId,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moot_reason_code: Option<MootReasonCode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective_id: Option<UbuId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<TaskAssignee>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<UbuId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<UbuTimestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_estimate: Option<TaskDurationEstimate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub correlation_groups: Vec<TaskCorrelationGroup>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preconditions: Option<UniversePrecondition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<TaskEffect>,
    pub provenance: Provenance,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Inline assignee shape from the canonical Identity schema. The identity id is
/// retained here so Task payloads do not need a separate lookup to identify it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskAssignee {
    pub id: UbuId,
    pub subject_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub kind: crate::core::identity::IdentityKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TaskDurationEstimate {
    Fixed {
        seconds: u64,
    },
    ShiftedLognormalP95 {
        min_seconds: u64,
        mode_seconds: u64,
        p95_seconds: u64,
    },
}

impl TaskDurationEstimate {
    pub fn validate(&self) -> crate::Result<()> {
        match self {
            Self::Fixed { seconds } if *seconds == 0 => Err(UbuError::InvalidTaskDurationEstimate),
            Self::ShiftedLognormalP95 {
                min_seconds,
                mode_seconds,
                p95_seconds,
            } if !(min_seconds < mode_seconds && mode_seconds < p95_seconds) => {
                Err(UbuError::InvalidTaskDurationEstimate)
            }
            _ => Ok(()),
        }
    }

    /// The planning kernel consumes one scalar. p95 is selected for shifted
    /// log-normal estimates because it is the contract's conservative completion bound.
    pub fn scalar_seconds(&self) -> u64 {
        match self {
            Self::Fixed { seconds } => *seconds,
            Self::ShiftedLognormalP95 { p95_seconds, .. } => *p95_seconds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskCorrelationGroup {
    pub group: String,
    pub strength: f64,
}

#[derive(Deserialize)]
#[serde(remote = "Task", deny_unknown_fields)]
struct TaskWire {
    pub id: UbuId,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moot_reason_code: Option<MootReasonCode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective_id: Option<UbuId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<TaskAssignee>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<UbuId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<UbuTimestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_estimate: Option<TaskDurationEstimate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub correlation_groups: Vec<TaskCorrelationGroup>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preconditions: Option<UniversePrecondition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<TaskEffect>,
    pub provenance: Provenance,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl<'de> Deserialize<'de> for Task {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = TaskWire::deserialize(deserializer)?;
        let task = Self {
            id: wire.id,
            title: wire.title,
            description: wire.description,
            status: wire.status,
            moot_reason_code: wire.moot_reason_code,
            objective_id: wire.objective_id,
            assignee: wire.assignee,
            blocked_by: wire.blocked_by,
            due_at: wire.due_at,
            duration_estimate: wire.duration_estimate,
            correlation_groups: wire.correlation_groups,
            preconditions: wire.preconditions,
            effects: wire.effects,
            provenance: wire.provenance,
            tags: wire.tags,
        };
        // Preserve the existing API boundary: lifecycle-invalid Tasks still
        // deserialize so callers can report the dedicated lifecycle error.
        task.validate_fields().map_err(D::Error::custom)?;
        Ok(task)
    }
}

impl Task {
    pub fn validate_fields(&self) -> crate::Result<()> {
        if let Some(assignee) = &self.assignee {
            assignee
                .id
                .require_object_type(ObjectType::Identity)
                .map_err(|_| UbuError::InvalidTaskAssignee)?;
        }
        let mut blocked = BTreeSet::new();
        for id in &self.blocked_by {
            id.require_object_type(ObjectType::Task)?;
            if *id == self.id {
                return Err(UbuError::TaskSelfBlocked);
            }
            if !blocked.insert(id) {
                return Err(UbuError::DuplicateTaskBlockedBy);
            }
        }
        if let Some(estimate) = &self.duration_estimate {
            estimate.validate()?;
        }
        let mut tags = BTreeSet::new();
        for tag in &self.tags {
            if tag.is_empty() {
                return Err(UbuError::EmptyTaskTag);
            }
            if !tags.insert(tag) {
                return Err(UbuError::DuplicateTaskTag);
            }
        }
        let mut groups = BTreeSet::new();
        for group in &self.correlation_groups {
            if !(0.0..=1.0).contains(&group.strength) || !group.strength.is_finite() {
                return Err(UbuError::InvalidTaskCorrelationStrength);
            }
            if !groups.insert(&group.group) {
                return Err(UbuError::DuplicateTaskCorrelationGroup);
            }
        }
        Ok(())
    }
}

impl Eq for Task {}

/// Optional predicted mutation of `UniverseState` if the Task succeeds.
///
/// Absent (`Task.effects == None`) means completion mutates nothing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskEffect {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_probability: Option<f64>,
    pub mutations: Vec<UniverseMutation>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn mutation(operation: &str, target: &str) -> UniverseMutation {
        UniverseMutation {
            operation: operation.to_owned(),
            target: target.to_owned(),
            payload: Some(json!("done")),
            note: None,
        }
    }

    #[test]
    fn task_effect_round_trips_when_present() {
        let effect = TaskEffect {
            success_probability: Some(0.85),
            mutations: vec![mutation("set_fact", "facts.ticket.status")],
        };

        let value = serde_json::to_value(&effect).expect("serializes");
        assert_eq!(
            value,
            json!({
                "success_probability": 0.85,
                "mutations": [
                    {"operation": "set_fact", "target": "facts.ticket.status", "payload": "done"}
                ]
            })
        );
        let parsed: TaskEffect = serde_json::from_value(value).expect("deserializes");
        assert_eq!(parsed, effect);
    }

    #[test]
    fn task_effect_omits_absent_success_probability() {
        let effect = TaskEffect {
            success_probability: None,
            mutations: vec![mutation("clear_fact", "facts.ticket.status")],
        };

        let value = serde_json::to_value(&effect).expect("serializes");
        assert!(value.get("success_probability").is_none());
        let parsed: TaskEffect = serde_json::from_value(value).expect("deserializes");
        assert_eq!(parsed, effect);
    }

    #[test]
    fn task_omits_effects_when_none() {
        let task = Task {
            id: UbuId::new(crate::id_registry::ObjectType::Task),
            title: "Example".to_owned(),
            description: None,
            status: TaskStatus::Active,
            moot_reason_code: None,
            objective_id: None,
            assignee: None,
            blocked_by: Vec::new(),
            due_at: None,
            duration_estimate: None,
            correlation_groups: Vec::new(),
            preconditions: None,
            effects: None,
            provenance: Provenance {
                created_at: UbuTimestamp::parse("2026-06-22T12:00:00Z").expect("valid timestamp"),
                created_by: None,
                authority_source: crate::authority::AuthoritySource::User,
                source: None,
                source_refs: None,
            },
            tags: Vec::new(),
        };

        let value = serde_json::to_value(&task).expect("serializes");
        assert!(value.get("effects").is_none());

        let parsed: Task = serde_json::from_value(value).expect("deserializes");
        assert_eq!(parsed, task);
    }

    fn base_task() -> Task {
        serde_json::from_str(include_str!(
            "../../fixtures/placeholders/valid/core/task/basic.json"
        ))
        .unwrap()
    }

    #[test]
    fn task_validation_rejects_blocking_and_tag_errors() {
        let mut task = base_task();
        task.blocked_by = vec![task.id.clone()];
        assert_eq!(task.validate_fields(), Err(UbuError::TaskSelfBlocked));
        let other = UbuId::new(crate::id_registry::ObjectType::Task);
        task.blocked_by = vec![other.clone(), other];
        assert_eq!(
            task.validate_fields(),
            Err(UbuError::DuplicateTaskBlockedBy)
        );
        task.blocked_by.clear();
        task.tags = vec![String::new()];
        assert_eq!(task.validate_fields(), Err(UbuError::EmptyTaskTag));
        task.tags = vec!["focus".into(), "focus".into()];
        assert_eq!(task.validate_fields(), Err(UbuError::DuplicateTaskTag));
    }

    #[test]
    fn task_validation_rejects_duration_and_correlation_errors() {
        let mut task = base_task();
        task.duration_estimate = Some(TaskDurationEstimate::Fixed { seconds: 0 });
        assert_eq!(
            task.validate_fields(),
            Err(UbuError::InvalidTaskDurationEstimate)
        );
        task.duration_estimate = Some(TaskDurationEstimate::ShiftedLognormalP95 {
            min_seconds: 10,
            mode_seconds: 10,
            p95_seconds: 20,
        });
        assert_eq!(
            task.validate_fields(),
            Err(UbuError::InvalidTaskDurationEstimate)
        );
        task.duration_estimate = None;
        task.correlation_groups = vec![
            TaskCorrelationGroup {
                group: "g".into(),
                strength: 0.5,
            },
            TaskCorrelationGroup {
                group: "g".into(),
                strength: 0.5,
            },
        ];
        assert_eq!(
            task.validate_fields(),
            Err(UbuError::DuplicateTaskCorrelationGroup)
        );
    }

    #[test]
    fn task_to_task_spec_projects_duration_models() {
        let mut task = base_task();
        assert!(crate::planning::TaskSpec::from(&task).estimate.is_none());
        task.duration_estimate = Some(TaskDurationEstimate::Fixed { seconds: 120 });
        assert_eq!(
            crate::planning::TaskSpec::from(&task)
                .estimate
                .unwrap()
                .seconds,
            120
        );
        task.duration_estimate = Some(TaskDurationEstimate::ShiftedLognormalP95 {
            min_seconds: 10,
            mode_seconds: 20,
            p95_seconds: 90,
        });
        assert_eq!(
            crate::planning::TaskSpec::from(&task)
                .estimate
                .unwrap()
                .seconds,
            90
        );
    }
}
