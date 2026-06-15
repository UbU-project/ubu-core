use serde::{Deserialize, Serialize};

use crate::compartment_label::CompartmentLabel;
use crate::policy_summary::PolicySummary;
use crate::UbuId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compartment {
    pub id: UbuId,
    pub label: CompartmentLabel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_summary: Option<PolicySummary>,
}
