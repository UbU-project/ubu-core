use crate::id_registry::ObjectType;
use crate::ids::UbuId;
use crate::time::UbuTimestamp;

pub fn validate_id(value: &str) -> crate::Result<UbuId> {
    UbuId::parse(value)
}

pub fn validate_id_for_type(value: &str, object_type: ObjectType) -> crate::Result<UbuId> {
    let id = UbuId::parse(value)?;
    id.require_object_type(object_type)?;
    Ok(id)
}

pub fn validate_timestamp(value: &str) -> crate::Result<UbuTimestamp> {
    UbuTimestamp::parse(value)
}
