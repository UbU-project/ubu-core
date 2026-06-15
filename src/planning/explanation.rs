use serde::{Deserialize, Serialize};

use crate::object_ref::ObjectRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplanationFragment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<ObjectRef>,
    pub text: String,
    pub severity: Severity,
}
