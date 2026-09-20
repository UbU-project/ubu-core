//! By-value local advisory boundary (UBU-D0255), with no execution or transport.

use std::collections::BTreeMap;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{AdvisoryCandidate, CandidateKind, DeviceId, ExecutionContext, UbuError, UbuTimestamp};

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
            (
                "expected_result_schema",
                !self.expected_result_schema.is_empty(),
            ),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputeBudget {
    pub max_cpu_ms: u64,
    pub max_memory_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    pub provider_name: String,
    pub provider_version: String,
    pub model_name: String,
    pub model_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_template_digest: Option<String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalAdvisoryResultStatus {
    Ok,
    Partial,
    Rejected,
    Timeout,
    WorkerError,
    MalformedResult,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "LocalAdvisoryResultWire")]
pub struct LocalAdvisoryResult {
    pub submission_id: String,
    pub authority: WorkerAuthority,
    pub provider_config: ProviderConfig,
    pub observed_policy_versions: BTreeMap<String, String>,
    pub input_digests: BTreeMap<String, String>,
    pub status: LocalAdvisoryResultStatus,
    #[serde(default)]
    pub artifacts: Vec<serde_json::Value>,
    #[serde(default)]
    pub proposed_candidates: Vec<AdvisoryCandidate>,
    #[serde(default)]
    pub diagnostics: Vec<serde_json::Value>,
    #[serde(default)]
    pub telemetry: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deletion_confirmed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalAdvisoryResultWire {
    pub submission_id: String,
    pub authority: WorkerAuthority,
    pub provider_config: ProviderConfig,
    pub observed_policy_versions: BTreeMap<String, String>,
    pub input_digests: BTreeMap<String, String>,
    pub status: LocalAdvisoryResultStatus,
    #[serde(default)]
    pub artifacts: Vec<serde_json::Value>,
    #[serde(default)]
    pub proposed_candidates: Vec<AdvisoryCandidate>,
    #[serde(default)]
    pub diagnostics: Vec<serde_json::Value>,
    #[serde(default)]
    pub telemetry: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deletion_confirmed: Option<bool>,
}

impl TryFrom<LocalAdvisoryResultWire> for LocalAdvisoryResult {
    type Error = UbuError;

    fn try_from(value: LocalAdvisoryResultWire) -> Result<Self, Self::Error> {
        if !matches!(
            value.status,
            LocalAdvisoryResultStatus::Ok | LocalAdvisoryResultStatus::Partial
        ) && !value.proposed_candidates.is_empty()
        {
            return Err(UbuError::InvalidLocalAdvisory {
                field: "proposed_candidates",
            });
        }
        Ok(Self {
            submission_id: value.submission_id,
            authority: value.authority,
            provider_config: value.provider_config,
            observed_policy_versions: value.observed_policy_versions,
            input_digests: value.input_digests,
            status: value.status,
            artifacts: value.artifacts,
            proposed_candidates: value.proposed_candidates,
            diagnostics: value.diagnostics,
            telemetry: value.telemetry,
            deletion_confirmed: value.deletion_confirmed,
        })
    }
}

impl LocalAdvisoryResult {
    /// A failed, cancelled, timed out, or malformed result can never enter the
    /// candidate queue. Ungranted candidates also fail closed as a whole result.
    pub fn admissible_candidates(&self) -> &[AdvisoryCandidate] {
        if !matches!(
            self.status,
            LocalAdvisoryResultStatus::Ok | LocalAdvisoryResultStatus::Partial
        ) || self
            .proposed_candidates
            .iter()
            .any(|candidate| !self.authority.may_propose(candidate.candidate_kind))
        {
            &[]
        } else {
            &self.proposed_candidates
        }
    }

    pub fn validate_against(&self, submission: &LocalAdvisorySubmission) -> crate::Result<()> {
        submission.validate()?;
        if self.submission_id != submission.submission_id {
            return Err(UbuError::InvalidLocalAdvisory {
                field: "submission_id",
            });
        }
        if self.authority != submission.authority {
            return Err(UbuError::InvalidLocalAdvisory { field: "authority" });
        }
        if self.provider_config != submission.provider_config {
            return Err(UbuError::InvalidLocalAdvisory {
                field: "provider_config",
            });
        }
        if self.observed_policy_versions != submission.observed_policy_versions {
            return Err(UbuError::InvalidLocalAdvisory {
                field: "observed_policy_versions",
            });
        }
        if self.input_digests != submission.input_digests {
            return Err(UbuError::InvalidLocalAdvisory {
                field: "input_digests",
            });
        }
        for candidate in &self.proposed_candidates {
            if !self.authority.may_propose(candidate.candidate_kind) {
                return Err(UbuError::UngrantedAdvisoryCapability {
                    kind: serde_json::to_string(&candidate.candidate_kind).unwrap(),
                });
            }
        }
        if self.status == LocalAdvisoryResultStatus::Partial && !submission.partial_results_allowed
        {
            return Err(UbuError::InvalidLocalAdvisory {
                field: "partial_results_allowed",
            });
        }
        if serde_json::to_vec(self)
            .map_err(|_| UbuError::AdvisoryResultTooLarge)?
            .len()
            > submission.result_size_limit_bytes as usize
        {
            return Err(UbuError::AdvisoryResultTooLarge);
        }
        if submission.authority.deadline.is_some() && self.deletion_confirmed.is_none() {
            return Err(UbuError::InvalidLocalAdvisory {
                field: "deletion_confirmed",
            });
        }
        Ok(())
    }
}

pub trait AdvisoryTransport {
    fn submit(&self, submission: &LocalAdvisorySubmission) -> crate::Result<LocalAdvisoryResult>;
}

#[cfg(test)]
pub struct StubAdvisoryTransport {
    pub result: LocalAdvisoryResult,
}

#[cfg(test)]
impl AdvisoryTransport for StubAdvisoryTransport {
    fn submit(&self, _submission: &LocalAdvisorySubmission) -> crate::Result<LocalAdvisoryResult> {
        Ok(self.result.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn authority(granted: BTreeSet<AdvisoryCapability>) -> WorkerAuthority {
        WorkerAuthority {
            worker_id: crate::UbuId::parse("worker_018f3c8e9b2a7c4d8f1e2a3b4c5d6e7f").unwrap(),
            authority_source: crate::AuthoritySource::AutomationWorker,
            granted,
            deadline: None,
        }
    }

    fn submission(authority: WorkerAuthority) -> LocalAdvisorySubmission {
        LocalAdvisorySubmission {
            submission_id: "submission-1".into(),
            authority,
            payload: serde_json::json!({}),
            expected_result_schema: "local-advisory-result".into(),
            timeout_ms: 1,
            compute_budget: ComputeBudget {
                max_cpu_ms: 1,
                max_memory_bytes: 1,
            },
            result_size_limit_bytes: 100_000,
            partial_results_allowed: true,
            causal_parents: vec![],
            observed_policy_versions: Default::default(),
            input_digests: Default::default(),
            provider_config: ProviderConfig {
                provider_name: "local".into(),
                provider_version: "1".into(),
                model_name: "m".into(),
                model_version: "1".into(),
                prompt_template_digest: None,
            },
            origin_device_id: DeviceId::parse("dev-1").unwrap(),
            execution_context: None,
            submitted_at: UbuTimestamp::parse("2026-09-19T09:00:00Z").unwrap(),
        }
    }

    fn candidate() -> AdvisoryCandidate {
        serde_json::from_str(include_str!(
            "../../fixtures/placeholders/valid/core/advisory-candidate/proposed-tag.json"
        ))
        .unwrap()
    }

    fn result(
        status: LocalAdvisoryResultStatus,
        candidate: AdvisoryCandidate,
        authority: WorkerAuthority,
    ) -> LocalAdvisoryResult {
        LocalAdvisoryResult {
            submission_id: "submission-1".into(),
            authority,
            provider_config: ProviderConfig {
                provider_name: "local".into(),
                provider_version: "1".into(),
                model_name: "m".into(),
                model_version: "1".into(),
                prompt_template_digest: None,
            },
            observed_policy_versions: Default::default(),
            input_digests: Default::default(),
            status,
            artifacts: vec![],
            proposed_candidates: vec![candidate],
            diagnostics: vec![],
            telemetry: vec![],
            deletion_confirmed: None,
        }
    }

    #[test]
    fn denied_capabilities_are_absent_and_default_authority_denies_all_kinds() {
        let authority = authority(BTreeSet::new());
        for kind in [
            CandidateKind::Tag,
            CandidateKind::Dependency,
            CandidateKind::Preference,
            CandidateKind::Decomposition,
            CandidateKind::ClarificationQuestion,
        ] {
            assert!(!authority.may_propose(kind));
        }
        // AdvisoryCapability has only proposal, diagnostics, and telemetry variants;
        // canonical writes, admission, state handles, fetch, export, network, tools,
        // credentials, and retention are intentionally not representable.
    }

    #[test]
    fn failure_statuses_never_admit_candidates() {
        let authority = authority(BTreeSet::from([AdvisoryCapability::ProposeCandidate(
            CandidateKind::Tag,
        )]));
        for status in [
            LocalAdvisoryResultStatus::Rejected,
            LocalAdvisoryResultStatus::Timeout,
            LocalAdvisoryResultStatus::WorkerError,
            LocalAdvisoryResultStatus::MalformedResult,
            LocalAdvisoryResultStatus::Cancelled,
        ] {
            assert!(result(status, candidate(), authority.clone())
                .admissible_candidates()
                .is_empty());
        }
    }

    #[test]
    fn validation_enforces_kind_partial_size_and_deletion() {
        let authority = authority(BTreeSet::from([AdvisoryCapability::ProposeCandidate(
            CandidateKind::Tag,
        )]));
        let base_submission = submission(authority.clone());
        let mut decomposition = candidate();
        decomposition.candidate_kind = CandidateKind::Decomposition;
        let err = result(
            LocalAdvisoryResultStatus::Ok,
            decomposition,
            authority.clone(),
        )
        .validate_against(&base_submission)
        .unwrap_err();
        assert!(err.to_string().contains("decomposition"));
        let mut partial_submission = base_submission.clone();
        partial_submission.partial_results_allowed = false;
        assert!(result(
            LocalAdvisoryResultStatus::Partial,
            candidate(),
            authority.clone()
        )
        .validate_against(&partial_submission)
        .is_err());
        let mut small = base_submission.clone();
        small.result_size_limit_bytes = 1;
        assert!(result(
            LocalAdvisoryResultStatus::Ok,
            candidate(),
            authority.clone()
        )
        .validate_against(&small)
        .is_err());
        let mut deadline_authority = authority;
        deadline_authority.deadline = Some(UbuTimestamp::parse("2026-09-20T00:00:00Z").unwrap());
        let deadline_submission = submission(deadline_authority.clone());
        assert!(result(
            LocalAdvisoryResultStatus::Ok,
            candidate(),
            deadline_authority
        )
        .validate_against(&deadline_submission)
        .is_err());
    }

    #[test]
    fn empty_grants_are_rejected() {
        assert!(submission(authority(BTreeSet::new())).validate().is_err());
    }
}
