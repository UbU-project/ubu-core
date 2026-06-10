use serde::{Deserialize, Serialize};

use crate::ids::UbuId;
use crate::provenance::Provenance;
use crate::source_ref::SourceRef;
use crate::time::UbuTimestamp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalReference {
    pub id: UbuId,
    pub source: SourceRef,
    pub title: String,
    pub observed_at: UbuTimestamp,
    pub provenance: Provenance,
}
