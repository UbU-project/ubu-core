use serde::{Deserialize, Serialize};

use crate::core::work_item::WorkItem;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    pub name: String,
    pub items: Vec<WorkItem>,
}
