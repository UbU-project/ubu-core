use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::authority::AuthoritySource;
use crate::{CandidateKind, UbuId, UbuTimestamp};

use super::local_advisory::AdvisoryCapability;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerAuthority {
    pub worker_id: UbuId,
    pub authority_source: AuthoritySource,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub granted: BTreeSet<AdvisoryCapability>,
    /// Retention ends at this deadline; this is never a retention grant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<UbuTimestamp>,
}

impl WorkerAuthority {
    /// Authority-source provenance does not itself grant any advisory power.
    pub fn may_propose(&self, kind: CandidateKind) -> bool {
        self.granted
            .contains(&AdvisoryCapability::ProposeCandidate(kind))
    }
}
