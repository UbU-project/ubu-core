use serde::{Deserialize, Serialize};

use crate::authority::AuthoritySource;
use crate::object_ref::ObjectRef;
use crate::time::UbuTimestamp;
use crate::UbuId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: UbuId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<ObjectRef>,
    pub message: String,
    pub logged_at: UbuTimestamp,
    pub authority_source: AuthoritySource,
}
