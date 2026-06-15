use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::authority::AuthoritySource;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preference {
    pub name: String,
    pub value: Value,
    pub authority_source: AuthoritySource,
}
