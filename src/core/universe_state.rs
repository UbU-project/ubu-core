use serde::{Deserialize, Serialize};

use crate::core::objective::Objective;
use crate::core::task::Task;
use crate::time::UbuTimestamp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniverseState {
    pub captured_at: UbuTimestamp,
    pub objectives: Vec<Objective>,
    pub tasks: Vec<Task>,
}
