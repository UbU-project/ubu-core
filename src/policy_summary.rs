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
    pub checked_at: UbuTimestamp,
}
