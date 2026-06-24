use serde::{Deserialize, Serialize};

use crate::core::universe_state::{UniverseMutation, UniversePrecondition};
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub due_at: Option<UbuTimestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preconditions: Option<UniversePrecondition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<TaskEffect>,
    pub provenance: Provenance,
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
            due_at: None,
            preconditions: None,
            effects: None,
            provenance: Provenance {
                created_at: UbuTimestamp::parse("2026-06-22T12:00:00Z").expect("valid timestamp"),
                created_by: None,
                authority_source: crate::authority::AuthoritySource::User,
                source: None,
                source_refs: None,
            },
        };

        let value = serde_json::to_value(&task).expect("serializes");
        assert!(value.get("effects").is_none());

        let parsed: Task = serde_json::from_value(value).expect("deserializes");
        assert_eq!(parsed, task);
    }
}
