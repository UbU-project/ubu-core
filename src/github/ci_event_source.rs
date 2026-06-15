use serde::{Deserialize, Serialize};

use crate::time::UbuTimestamp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubCiEventSource {
    pub repository: String,
    pub run_id: u64,
    pub status: String,
    pub html_url: String,
    pub updated_at: UbuTimestamp,
}
