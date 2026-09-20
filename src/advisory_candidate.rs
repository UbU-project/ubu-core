//! Advisory review-queue types, separate from admitted objects (UBU-D0274).

use std::collections::{BTreeMap, BTreeSet};

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::{
    CompartmentLabel, DeviceId, ExecutionContext, IdempotencyKey, ObjectRef, UbuError, UbuId,
    UbuTimestamp,
};

/// Candidate-state identity, deliberately outside the admitted-object registry.
/// Parsing accepts only the canonical lowercase, unhyphenated UUIDv7 spelling.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct AdvisoryCandidateId(String);

impl AdvisoryCandidateId {
    pub fn parse(value: impl Into<String>) -> crate::Result<Self> {
        let value = value.into();
        let valid = value.strip_prefix("advcand_").is_some_and(|suffix| {
            suffix.len() == 32
                && suffix
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                && Uuid::parse_str(suffix).is_ok_and(|uuid| {
                    uuid.get_version_num() == 7 && uuid.get_variant() == uuid::Variant::RFC4122
                })
        });
        if !valid {
            return Err(UbuError::InvalidAdvisoryCandidateId { value });
        }
        Ok(Self(value))
    }

    pub fn generate() -> Self {
        Self(format!("advcand_{}", Uuid::now_v7().simple()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for AdvisoryCandidateId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateKind {
    Tag,
    Dependency,
    Preference,
    Decomposition,
    ClarificationQuestion,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateLifecycleState {
    Proposed,
    Deferred,
    Resurfaced,
    Admitted,
    Rejected,
    Superseded,
    Archived,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResurfaceTrigger {
    MateriallyNewEvidence,
    UserRequest,
    PolicyReviewInterval,
    AcceptedChangeToTargetOrDependencies,
    ClarificationOrExternalReferenceArrival,
}

/// A disposition, not a duration or an automatic purge implementation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionPolicy {
    /// Retain under the applicable Compartment policy.
    Retain,
    /// Payload may be purged while durable correction metadata survives.
    PurgePayload,
}

/// Review visibility restriction; neither variant grants export authority.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisclosurePolicy {
    CompartmentOnly,
    RedactedOnly,
}

impl CandidateLifecycleState {
    pub const INITIAL: Self = Self::Proposed;

    /// Structural edges only; `transition` additionally checks trigger placement.
    pub fn can_transition_to(self, next: Self) -> bool {
        use CandidateLifecycleState::*;
        matches!(
            (self, next),
            (
                Proposed | Resurfaced,
                Admitted | Rejected | Deferred | Superseded | Archived
            ) | (Deferred, Resurfaced | Rejected | Superseded | Archived)
                | (Admitted | Rejected | Superseded, Archived)
        )
    }

    pub fn is_terminal(self) -> bool {
        self == Self::Archived
    }

    pub fn is_active_queue(self) -> bool {
        matches!(self, Self::Proposed | Self::Resurfaced)
    }
}

/// Apply exactly one UBU-D0274 edge, with a trigger only when resurfacing.
pub fn transition(
    current: CandidateLifecycleState,
    next: CandidateLifecycleState,
    trigger: Option<ResurfaceTrigger>,
) -> crate::Result<CandidateLifecycleState> {
    if !current.can_transition_to(next) {
        return Err(UbuError::InvalidCandidateTransition { current, next });
    }
    if (next == CandidateLifecycleState::Resurfaced) != trigger.is_some() {
        return Err(UbuError::InvalidResurfaceTrigger { current, next });
    }
    Ok(next)
}

/// Explicit tags keep redacted summaries distinct from arbitrary inline JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum CandidatePayload {
    Inline(serde_json::Value),
    RedactedSummary(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ReviewLabel {
    Label(CompartmentLabel),
    Redacted,
}

/// Advisory model/tool provenance, never an authority grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposingActor {
    pub model_or_tool_name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_template_digest: Option<String>,
}

/// Opaque references: decision-event identities are intentionally deferred to P1B-5.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateLinks {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deferral_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_deferral_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resurface_trigger: Option<ResurfaceTrigger>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trigger_evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resurfacing_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resurfacing_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersession_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correction_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_ref: Option<String>,
}

/// A versioned candidate-state record; this type does not admit or persist objects.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AdvisoryCandidate {
    pub advisory_candidate_id: AdvisoryCandidateId,
    pub schema_version: String,
    pub candidate_kind: CandidateKind,
    pub lifecycle_state: CandidateLifecycleState,
    pub version: u64,
    /// Typed references jointly describe target or scope, as in the ticket sketch.
    pub target_refs: Vec<ObjectRef>,
    pub normalized_proposal: serde_json::Value,
    pub payload: CandidatePayload,
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    pub field_provenance: BTreeMap<String, String>,
    pub proposed_at: UbuTimestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_time: Option<UbuTimestamp>,
    pub proposing_actor: ProposingActor,
    pub origin_device_id: DeviceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_context: Option<ExecutionContext>,
    pub idempotency_key: IdempotencyKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppression_key: Option<String>,
    #[serde(deserialize_with = "deserialize_unique_ids")]
    pub compartment_ids: BTreeSet<UbuId>,
    pub review_label: ReviewLabel,
    pub disclosure_policy: DisclosurePolicy,
    pub retention_policy: RetentionPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_order: Option<i64>,
    pub links: CandidateLinks,
}

// Remote derive shares the public layout, then checks cross-field invariants.
#[derive(Deserialize)]
#[serde(remote = "AdvisoryCandidate", deny_unknown_fields)]
struct AdvisoryCandidateWire {
    pub advisory_candidate_id: AdvisoryCandidateId,
    pub schema_version: String,
    pub candidate_kind: CandidateKind,
    pub lifecycle_state: CandidateLifecycleState,
    pub version: u64,
    /// Typed references jointly describe target or scope, as in the ticket sketch.
    pub target_refs: Vec<ObjectRef>,
    pub normalized_proposal: serde_json::Value,
    pub payload: CandidatePayload,
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    pub field_provenance: BTreeMap<String, String>,
    pub proposed_at: UbuTimestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_time: Option<UbuTimestamp>,
    pub proposing_actor: ProposingActor,
    pub origin_device_id: DeviceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_context: Option<ExecutionContext>,
    pub idempotency_key: IdempotencyKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppression_key: Option<String>,
    #[serde(deserialize_with = "deserialize_unique_ids")]
    pub compartment_ids: BTreeSet<UbuId>,
    pub review_label: ReviewLabel,
    pub disclosure_policy: DisclosurePolicy,
    pub retention_policy: RetentionPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_order: Option<i64>,
    pub links: CandidateLinks,
}

impl<'de> Deserialize<'de> for AdvisoryCandidate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let candidate = AdvisoryCandidateWire::deserialize(deserializer)?;
        candidate.validate().map_err(D::Error::custom)?;
        Ok(candidate)
    }
}

fn deserialize_unique_ids<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeSet<UbuId>, D::Error> {
    let mut ids = BTreeSet::new();
    for id in Vec::<UbuId>::deserialize(deserializer)? {
        if !ids.insert(id) {
            return Err(D::Error::custom("duplicate Compartment id"));
        }
    }
    Ok(ids)
}

impl AdvisoryCandidate {
    pub fn validate(&self) -> crate::Result<()> {
        if self
            .confidence
            .is_some_and(|value| !(0.0..=1.0).contains(&value))
        {
            return Err(UbuError::InvalidCandidateRecord {
                field: "confidence",
            });
        }
        if self.version == 0 {
            return Err(UbuError::InvalidCandidateRecord { field: "version" });
        }
        if self.schema_version.is_empty() {
            return Err(UbuError::InvalidCandidateRecord {
                field: "schema_version",
            });
        }
        for id in &self.compartment_ids {
            // The sketch requests parseable UbuIds, not a new Compartment-only constraint.
            UbuId::parse(id.as_str())?;
        }
        if self.lifecycle_state == CandidateLifecycleState::Resurfaced
            && (self
                .links
                .prior_deferral_ref
                .as_ref()
                .is_none_or(String::is_empty)
                || self.links.resurface_trigger.is_none())
        {
            return Err(UbuError::InvalidCandidateRecord { field: "links" });
        }
        Ok(())
    }
}
