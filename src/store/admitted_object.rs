use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::time::UbuTimestamp;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdmittedObject {
    pub object: Value,
    pub admitted_at: UbuTimestamp,
}
