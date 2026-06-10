use serde::{Deserialize, Serialize};

use crate::time::UbuTimestamp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecalculationTrigger {
    pub triggered_at: UbuTimestamp,
    pub reason: String,
}
