use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Duration {
    pub seconds: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iso8601: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub currency: String,
    pub amount_minor: i64,
}
