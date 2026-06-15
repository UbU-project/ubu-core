use crate::core::{Task, TaskStatus};
use crate::errors::UbuError;
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

pub fn validate_task_lifecycle(task: &Task) -> crate::Result<()> {
    match (task.status, task.moot_reason_code) {
        (TaskStatus::Moot, None) => Err(UbuError::MissingMootReasonCode),
        (TaskStatus::Moot, Some(_)) => Ok(()),
        (status, Some(_)) => Err(UbuError::UnexpectedMootReasonCode {
            status: status.as_str(),
        }),
        (_, None) => Ok(()),
    }
}
