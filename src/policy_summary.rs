use serde::{Deserialize, Serialize};

use crate::time::UbuTimestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Legitimization {
    Accepted,
    NeedsReview,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicySummary {
    pub legitimization: Legitimization,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adjudication_reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_only: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_cloud_llm: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_external_export: Option<bool>,
    pub checked_at: UbuTimestamp,
}
