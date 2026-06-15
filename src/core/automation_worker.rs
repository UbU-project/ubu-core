use serde::{Deserialize, Serialize};

use crate::time::UbuTimestamp;
use crate::UbuId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationWorker {
    pub id: UbuId,
    pub name: String,
    pub capabilities: Vec<String>,
    pub registered_at: UbuTimestamp,
}
