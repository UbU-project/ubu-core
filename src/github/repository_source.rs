use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubRepositorySource {
    pub owner: String,
    pub name: String,
    pub html_url: String,
}
