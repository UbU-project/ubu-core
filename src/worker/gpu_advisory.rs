use serde::{Deserialize, Serialize};

use crate::serde_helpers::{Duration, Money};
use crate::time::UbuTimestamp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuAdvisoryRequest {
    pub request_id: String,
    pub requested_at: UbuTimestamp,
    pub workload: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_duration: Option<Duration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_cost: Option<Money>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuAdvisoryRecommendation {
    RunNow,
    Defer,
    Split,
    ManualReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuAdvisoryResponse {
    pub request_id: String,
    pub responded_at: UbuTimestamp,
    pub recommendation: GpuAdvisoryRecommendation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_cost: Option<Money>,
}
