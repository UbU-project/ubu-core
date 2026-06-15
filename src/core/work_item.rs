use serde::{Deserialize, Serialize};

use crate::object_ref::ObjectRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItem {
    #[serde(rename = "ref")]
    pub object_ref: ObjectRef,
    pub summary: String,
}
