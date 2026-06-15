use serde::{Deserialize, Serialize};

use crate::id_registry::ObjectType;
use crate::ids::UbuId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectRef {
    pub id: UbuId,
    pub object_type: ObjectType,
}
