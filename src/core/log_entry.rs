use serde::{Deserialize, Serialize};

use crate::authority::AuthoritySource;
use crate::object_ref::ObjectRef;
use crate::policy_summary::Legitimization;
use crate::provenance::Provenance;
use crate::time::UbuTimestamp;
use crate::UbuId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEventType {
    CompartmentBoundaryDecided,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMember {
    LocalOnly,
    NoCloudLlm,
    NoExternalExport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompartmentBoundaryDecidedPayload {
    pub compartment_ref: ObjectRef,
    pub member_evaluated: PolicyMember,
    pub adjudication_result: Legitimization,
    pub actor_identity_ref: ObjectRef,
    pub authority_source: AuthoritySource,
    pub reason: String,
    pub effective_time: UbuTimestamp,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: UbuId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<ObjectRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_type: Option<LogEventType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<CompartmentBoundaryDecidedPayload>,
    pub logged_at: UbuTimestamp,
    pub authority_source: AuthoritySource,
}
