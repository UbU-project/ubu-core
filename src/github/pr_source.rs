use serde::{Deserialize, Serialize};

use crate::time::UbuTimestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitHubPrState {
    Open,
    Closed,
    Merged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubPrSource {
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub state: GitHubPrState,
    pub html_url: String,
    pub updated_at: UbuTimestamp,
}
