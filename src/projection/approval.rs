use serde::{Deserialize, Serialize};

use crate::authority::AuthoritySource;
use crate::time::UbuTimestamp;
use crate::UbuId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionApproval {
    pub preview_id: UbuId,
    pub approved: bool,
    pub approved_at: UbuTimestamp,
    pub authority_source: AuthoritySource,
}
