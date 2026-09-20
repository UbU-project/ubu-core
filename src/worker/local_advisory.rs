//! By-value local advisory boundary (UBU-D0255), with no execution or transport.

use std::collections::BTreeMap;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{CandidateKind, DeviceId, ExecutionContext, UbuError, UbuTimestamp};

use super::WorkerAuthority;

/// The complete grant vocabulary. There is no write, admission, StateStore,
/// context-fetch, authority-expansion, export-policy, network, tool, credential,
/// or retention capability. Downstream code cannot add enum variants.
/// `#[non_exhaustive]` permits future library evolution, not caller-defined grants.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryCapability {
    ProposeCandidate(CandidateKind),
    EmitDiagnostics,
    EmitTelemetry,
}

/// Minimized, owned input. No state handle, callback, tool, or credential slot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LocalAdvisorySubmission {
    pub submission_id: String,
    pub authority: WorkerAuthority,
    pub payload: serde_json::Value,
    pub expected_result_schema: String,
    pub timeout_ms: u64,
    pub compute_budget: ComputeBudget,
    pub result_size_limit_bytes: u64,
    pub partial_results_allowed: bool,
    pub causal_parents: Vec<String>,
    pub observed_policy_versions: BTreeMap<String, String>,
    pub input_digests: BTreeMap<String, String>,
    pub provider_config: ProviderConfig,
    pub origin_device_id: DeviceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_context: Option<ExecutionContext>,
    pub submitted_at: UbuTimestamp,
}

#[derive(Deserialize)]
#[serde(remote = "LocalAdvisorySubmission", deny_unknown_fields)]
struct LocalAdvisorySubmissionWire {
    pub submission_id: String,
    pub authority: WorkerAuthority,
    pub payload: serde_json::Value,
    pub expected_result_schema: String,
    pub timeout_ms: u64,
    pub compute_budget: ComputeBudget,
    pub result_size_limit_bytes: u64,
    pub partial_results_allowed: bool,
    pub causal_parents: Vec<String>,
    pub observed_policy_versions: BTreeMap<String, String>,
    pub input_digests: BTreeMap<String, String>,
    pub provider_config: ProviderConfig,
    pub origin_device_id: DeviceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_context: Option<ExecutionContext>,
    pub submitted_at: UbuTimestamp,
}

impl<'de> Deserialize<'de> for LocalAdvisorySubmission {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let submission = LocalAdvisorySubmissionWire::deserialize(deserializer)?;
        submission.validate().map_err(D::Error::custom)?;
        Ok(submission)
    }
}

impl LocalAdvisorySubmission {
    pub fn validate(&self) -> crate::Result<()> {
        for (field, valid) in [
            ("submission_id", !self.submission_id.is_empty()),
            ("expected_result_schema", !self.expected_result_schema.is_empty()),
            ("timeout_ms", self.timeout_ms != 0),
            ("result_size_limit_bytes", self.result_size_limit_bytes != 0),
            ("authority.granted", !self.authority.granted.is_empty()),
        ] {
            if !valid {
                return Err(UbuError::InvalidLocalAdvisory { field });
            }
        }
        Ok(())
    }
}
