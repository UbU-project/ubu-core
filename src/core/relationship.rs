use serde::{Deserialize, Serialize};

use crate::object_ref::ObjectRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Relationship {
    pub from: ObjectRef,
    pub to: ObjectRef,
    pub kind: String,
}
