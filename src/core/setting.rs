use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::authority::AuthoritySource;
use crate::ids::UbuId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Setting {
    pub id: UbuId,
    pub name: String,
    pub value: Value,
    pub authority_source: AuthoritySource,
}
