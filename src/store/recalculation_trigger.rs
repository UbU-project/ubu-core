use serde::{Deserialize, Serialize};

use crate::object_ref::ObjectRef;
use crate::time::UbuTimestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    TaskCompleted,
    TaskFailed,
    TaskMoot,
    UserOverride,
    ObservedSnapshot,
    ExternalEvent,
    GithubUpdate,
    LowCompactCalendarCoverage,
    WorkerRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecalculationTrigger {
    pub triggered_at: UbuTimestamp,
    pub trigger_type: TriggerType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<ObjectRef>,
}
