use serde::{Deserialize, Serialize};

use crate::time::UbuTimestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitHubIssueState {
    Open,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubIssueSource {
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub state: GitHubIssueState,
    pub html_url: String,
    pub updated_at: UbuTimestamp,
}
