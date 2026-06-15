use serde::{Deserialize, Serialize};

use crate::object_ref::ObjectRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relationship {
    pub from: ObjectRef,
    pub to: ObjectRef,
    pub kind: String,
}
