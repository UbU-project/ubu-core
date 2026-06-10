use serde::{Deserialize, Serialize};

use crate::authority::AuthoritySource;
use crate::source_ref::SourceRef;
use crate::time::UbuTimestamp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    pub created_at: UbuTimestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    pub authority_source: AuthoritySource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceRef>,
}
