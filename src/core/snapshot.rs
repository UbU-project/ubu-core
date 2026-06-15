use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::time::UbuTimestamp;
use crate::UbuId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: UbuId,
    pub captured_at: UbuTimestamp,
    pub objects: Vec<Value>,
}
