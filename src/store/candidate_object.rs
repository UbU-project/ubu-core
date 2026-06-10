use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::authority::AuthoritySource;
use crate::time::UbuTimestamp;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateObject {
    pub candidate_id: String,
    pub object_type: String,
    pub payload: Value,
    pub submitted_at: UbuTimestamp,
    pub authority_source: AuthoritySource,
}
