use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::source_ref::SourceRef;
use crate::time::UbuTimestamp;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalEvent {
    pub source: SourceRef,
    pub event_type: String,
    pub occurred_at: UbuTimestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}
