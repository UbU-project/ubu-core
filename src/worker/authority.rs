use serde::{Deserialize, Serialize};

use crate::authority::AuthoritySource;
use crate::UbuId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerAuthority {
    pub worker_id: UbuId,
    pub authority_source: AuthoritySource,
}
