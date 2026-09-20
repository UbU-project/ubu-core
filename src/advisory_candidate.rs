//! Advisory review-queue types, separate from admitted objects (UBU-D0274).

use std::collections::{BTreeMap, BTreeSet};

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::{
    AuthoritySource, CompartmentLabel, DeviceId, ExecutionContext, IdempotencyKey, ObjectRef,
    ObjectType, UbuError, UbuId, UbuTimestamp,
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

/// Durable correction metadata. A suppression record must never make the rejected
/// proposal true, accepted, exportable, more visible, or eligible as evidence for
/// admitted state. It intentionally does not retain the rejected payload.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SuppressionRecord {
    pub candidate_kind: CandidateKind,
    pub normalized_proposal: serde_json::Value,
    /// Target/scope shape represented by the candidate's typed target-or-scope refs.
    pub target_and_scope_shape: Vec<ObjectRef>,
    #[serde(deserialize_with = "deserialize_unique_ids")]
    pub compartment_ids: BTreeSet<UbuId>,
    /// Label versus Redacted is the Compartment/redaction class.
    pub review_label: ReviewLabel,
    pub evidence_hashes_or_source_fingerprints: Vec<String>,
    /// Extractor/model/tool name and version, plus optional prompt/template digest.
    pub proposing_actor: ProposingActor,
    pub schema_version: String,
    pub rejection_reason_or_user_correction: String,
    pub deciding_actor_identity_id: UbuId,
    pub authority_source: AuthoritySource,
    pub decided_at: UbuTimestamp,
    pub retention_policy: RetentionPolicy,
    pub suppression_key: String,
}

#[derive(Deserialize)]
#[serde(remote = "SuppressionRecord", deny_unknown_fields)]
struct SuppressionRecordWire {
    pub candidate_kind: CandidateKind,
    pub normalized_proposal: serde_json::Value,
    /// Target/scope shape represented by the candidate's typed target-or-scope refs.
    pub target_and_scope_shape: Vec<ObjectRef>,
    #[serde(deserialize_with = "deserialize_unique_ids")]
    pub compartment_ids: BTreeSet<UbuId>,
    /// Label versus Redacted is the Compartment/redaction class.
    pub review_label: ReviewLabel,
    pub evidence_hashes_or_source_fingerprints: Vec<String>,
    /// Extractor/model/tool name and version, plus optional prompt/template digest.
    pub proposing_actor: ProposingActor,
    pub schema_version: String,
    pub rejection_reason_or_user_correction: String,
    pub deciding_actor_identity_id: UbuId,
    pub authority_source: AuthoritySource,
    pub decided_at: UbuTimestamp,
    pub retention_policy: RetentionPolicy,
    pub suppression_key: String,
}

impl<'de> Deserialize<'de> for SuppressionRecord {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let record = SuppressionRecordWire::deserialize(deserializer)?;
        record.validate().map_err(D::Error::custom)?;
        Ok(record)
    }
}

impl SuppressionRecord {
    pub fn validate(&self) -> crate::Result<()> {
        self.deciding_actor_identity_id
            .require_object_type(ObjectType::Identity)?;
        for (field, value) in [
            ("schema_version", &self.schema_version),
            (
                "rejection_reason_or_user_correction",
                &self.rejection_reason_or_user_correction,
            ),
            ("suppression_key", &self.suppression_key),
        ] {
            if value.is_empty() {
                return Err(UbuError::InvalidSuppressionRecord { field });
            }
        }
        for id in &self.compartment_ids {
            UbuId::parse(id.as_str())?;
        }
        Ok(())
    }
}

/// Decision inputs that cannot be inferred from advisory execution provenance.
/// Fingerprints/hashes must be supplied explicitly; evidence refs are not hashes.
#[derive(Debug, Clone, PartialEq)]
pub struct SuppressionDecision {
    pub deciding_actor_identity_id: UbuId,
    pub authority_source: AuthoritySource,
    pub decided_at: UbuTimestamp,
    pub rejection_reason_or_user_correction: String,
    pub retention_policy: RetentionPolicy,
    pub evidence_hashes_or_source_fingerprints: Vec<String>,
}

impl AdvisoryCandidate {
    /// Build durable correction metadata only from a rejected candidate with a
    /// suppression key. The rejected payload may afterwards be purged or redacted.
    /// This pure function neither performs retention nor grants admission authority.
    pub fn suppression_record(
        &self,
        decision: SuppressionDecision,
    ) -> crate::Result<SuppressionRecord> {
        self.validate()?;
        if self.lifecycle_state != CandidateLifecycleState::Rejected {
            return Err(UbuError::CandidateNotRejected {
                state: self.lifecycle_state,
            });
        }
        let suppression_key =
            self.suppression_key
                .clone()
                .ok_or(UbuError::InvalidSuppressionRecord {
                    field: "suppression_key",
                })?;
        let record = SuppressionRecord {
            candidate_kind: self.candidate_kind,
            normalized_proposal: self.normalized_proposal.clone(),
            target_and_scope_shape: self.target_refs.clone(),
            compartment_ids: self.compartment_ids.clone(),
            review_label: self.review_label.clone(),
            evidence_hashes_or_source_fingerprints: decision.evidence_hashes_or_source_fingerprints,
            proposing_actor: self.proposing_actor.clone(),
            schema_version: self.schema_version.clone(),
            rejection_reason_or_user_correction: decision.rejection_reason_or_user_correction,
            deciding_actor_identity_id: decision.deciding_actor_identity_id,
            authority_source: decision.authority_source,
            decided_at: decision.decided_at,
            retention_policy: decision.retention_policy,
            suppression_key,
        };
        record.validate()?;
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const STATES: [CandidateLifecycleState; 7] = [
        CandidateLifecycleState::Proposed,
        CandidateLifecycleState::Deferred,
        CandidateLifecycleState::Resurfaced,
        CandidateLifecycleState::Admitted,
        CandidateLifecycleState::Rejected,
        CandidateLifecycleState::Superseded,
        CandidateLifecycleState::Archived,
    ];

    fn candidate() -> AdvisoryCandidate {
        serde_json::from_str(include_str!(
            "../fixtures/placeholders/valid/core/advisory-candidate/proposed-tag.json"
        ))
        .unwrap()
    }

    fn decision() -> SuppressionDecision {
        SuppressionDecision {
            deciding_actor_identity_id: UbuId::parse("identity_018f3c8e9b2a7c4d8f1e2a3b4c5d6e7f")
                .unwrap(),
            authority_source: AuthoritySource::User,
            decided_at: UbuTimestamp::parse("2026-09-19T10:00:00Z").unwrap(),
            rejection_reason_or_user_correction: "Do not infer focus tags from this description."
                .into(),
            retention_policy: RetentionPolicy::PurgePayload,
            evidence_hashes_or_source_fingerprints: vec![
                "source-fingerprint:task-description:v1".into()
            ],
        }
    }

    #[test]
    fn every_lifecycle_pair_matches_the_exact_decision_matrix() {
        // Rows/columns use STATES order; all 49 pairs, including self-edges.
        let allowed = [
            [false, true, false, true, true, true, true],
            [false, false, true, false, true, true, true],
            [false, true, false, true, true, true, true],
            [false, false, false, false, false, false, true],
            [false, false, false, false, false, false, true],
            [false, false, false, false, false, false, true],
            [false, false, false, false, false, false, false],
        ];
        for (row, current) in STATES.into_iter().enumerate() {
            for (column, next) in STATES.into_iter().enumerate() {
                let expected = allowed[row][column];
                assert_eq!(
                    current.can_transition_to(next),
                    expected,
                    "{current:?} -> {next:?}"
                );
                let trigger = (next == CandidateLifecycleState::Resurfaced)
                    .then_some(ResurfaceTrigger::UserRequest);
                let result = transition(current, next, trigger);
                if expected {
                    assert_eq!(result, Ok(next));
                } else {
                    assert_eq!(
                        result,
                        Err(UbuError::InvalidCandidateTransition { current, next })
                    );
                }
            }
        }
        use CandidateLifecycleState::*;
        assert!(transition(Deferred, Admitted, None).is_err());
        assert!(transition(Rejected, Proposed, None).is_err());
        for next in STATES {
            assert!(transition(Archived, next, None).is_err());
        }
    }

    #[test]
    fn resurfacing_requires_one_of_the_five_triggers_only_on_that_edge() {
        use CandidateLifecycleState::*;
        let triggers = [
            ResurfaceTrigger::MateriallyNewEvidence,
            ResurfaceTrigger::UserRequest,
            ResurfaceTrigger::PolicyReviewInterval,
            ResurfaceTrigger::AcceptedChangeToTargetOrDependencies,
            ResurfaceTrigger::ClarificationOrExternalReferenceArrival,
        ];
        assert_eq!(
            transition(Deferred, Resurfaced, None),
            Err(UbuError::InvalidResurfaceTrigger {
                current: Deferred,
                next: Resurfaced,
            })
        );
        for trigger in triggers {
            assert_eq!(
                transition(Deferred, Resurfaced, Some(trigger)),
                Ok(Resurfaced)
            );
            for current in STATES {
                for next in STATES {
                    if current.can_transition_to(next) && next != Resurfaced {
                        assert_eq!(
                            transition(current, next, Some(trigger)),
                            Err(UbuError::InvalidResurfaceTrigger { current, next })
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn initial_terminal_and_queue_membership_are_exact() {
        use CandidateLifecycleState::*;
        assert_eq!(CandidateLifecycleState::INITIAL, Proposed);
        for state in STATES {
            assert_eq!(state.is_terminal(), state == Archived);
            assert_eq!(
                state.is_active_queue(),
                [Proposed, Resurfaced].contains(&state)
            );
        }
    }

    #[test]
    fn confidence_bounds_reject_nonfinite_and_out_of_range_values() {
        for confidence in [1.5, -0.1, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut value = candidate();
            value.confidence = Some(confidence);
            assert_eq!(
                value.validate(),
                Err(UbuError::InvalidCandidateRecord {
                    field: "confidence"
                })
            );
        }
        for confidence in [None, Some(0.0), Some(1.0)] {
            let mut value = candidate();
            value.confidence = confidence;
            value.validate().unwrap();
            let bytes = serde_json::to_vec(&value).unwrap();
            assert_eq!(
                serde_json::from_slice::<AdvisoryCandidate>(&bytes).unwrap(),
                value
            );
        }
    }

    #[test]
    fn candidate_validation_requires_version_schema_and_resurfacing_links() {
        let mut value = candidate();
        value.version = 0;
        assert_eq!(
            value.validate(),
            Err(UbuError::InvalidCandidateRecord { field: "version" })
        );
        assert!(
            serde_json::from_value::<AdvisoryCandidate>(serde_json::to_value(&value).unwrap())
                .is_err()
        );
        value.version = 1;
        value.schema_version.clear();
        assert_eq!(
            value.validate(),
            Err(UbuError::InvalidCandidateRecord {
                field: "schema_version"
            })
        );
        value.schema_version = "1.0".into();
        value.lifecycle_state = CandidateLifecycleState::Resurfaced;
        value.links.resurface_trigger = Some(ResurfaceTrigger::UserRequest);
        for prior in [None, Some(String::new())] {
            value.links.prior_deferral_ref = prior;
            assert_eq!(
                value.validate(),
                Err(UbuError::InvalidCandidateRecord { field: "links" })
            );
            assert!(serde_json::from_value::<AdvisoryCandidate>(
                serde_json::to_value(&value).unwrap()
            )
            .is_err());
        }
        value.links.prior_deferral_ref = Some("decision:defer:001".into());
        value.validate().unwrap();
        value.links.resurface_trigger = None;
        assert!(value.validate().is_err());
    }

    #[test]
    fn candidate_wire_rejects_bad_or_duplicate_ids_and_unknown_fields() {
        for ids in [
            json!(["not-an-id"]),
            json!([
                "comp_018f3c8e9b2a7c4d8f1e2a3b4c5d6e7f",
                "comp_018f3c8e9b2a7c4d8f1e2a3b4c5d6e7f"
            ]),
        ] {
            let mut value = serde_json::to_value(candidate()).unwrap();
            value["compartment_ids"] = ids;
            assert!(serde_json::from_value::<AdvisoryCandidate>(value).is_err());
        }
        let mut value = serde_json::to_value(candidate()).unwrap();
        value["admitted"] = json!(true);
        assert!(serde_json::from_value::<AdvisoryCandidate>(value).is_err());
    }

    #[test]
    fn generated_candidate_ids_are_unique_v7_and_outside_admitted_registry() {
        let mut ids = BTreeSet::new();
        for _ in 0..256 {
            let id = AdvisoryCandidateId::generate();
            let suffix = id.as_str().strip_prefix("advcand_").unwrap();
            assert_eq!(suffix.len(), 32);
            assert_eq!(Uuid::parse_str(suffix).unwrap().get_version_num(), 7);
            assert_eq!(AdvisoryCandidateId::parse(id.as_str()).unwrap(), id);
            assert_eq!(
                serde_json::from_value::<AdvisoryCandidateId>(json!(id)).unwrap(),
                id
            );
            assert!(UbuId::parse(id.as_str()).is_err());
            assert!(ids.insert(id));
        }
        for invalid in [
            "",
            "advcand_",
            "task_018f3c8e9b2a7c4d8f1e2a3b4c5d6e7f",
            "advcand_018f3c8e9b2a4c4d8f1e2a3b4c5d6e7f",
            "advcand_018f3c8e9b2a7c4d0f1e2a3b4c5d6e7f",
            "advcand_018F3C8E9B2A7C4D8F1E2A3B4C5D6E7F",
            "advcand_018f3c8e-9b2a-7c4d-8f1e-2a3b4c5d6e7f",
            "advcand_018f3c8e9b2a7c4d8f1e2a3b4c5d6e7f\n",
        ] {
            assert!(matches!(
                AdvisoryCandidateId::parse(invalid),
                Err(UbuError::InvalidAdvisoryCandidateId { .. })
            ));
        }
    }

    #[test]
    fn suppression_builder_preserves_metadata_and_omits_rejected_payload() {
        let mut candidate = candidate();
        candidate.lifecycle_state = CandidateLifecycleState::Rejected;
        let before = candidate.clone();
        let record = candidate.suppression_record(decision()).unwrap();
        assert_eq!(candidate, before);
        assert_eq!(record.candidate_kind, candidate.candidate_kind);
        assert_eq!(record.normalized_proposal, candidate.normalized_proposal);
        assert_eq!(
            Some(&record.suppression_key),
            candidate.suppression_key.as_ref()
        );
        assert_eq!(record.target_and_scope_shape, candidate.target_refs);
        assert_eq!(record.compartment_ids, candidate.compartment_ids);
        assert_eq!(record.review_label, candidate.review_label);
        assert_eq!(record.proposing_actor, candidate.proposing_actor);
        let expected = include_str!(
            "../fixtures/placeholders/valid/core/suppression-record/rejected-tag.json"
        );
        assert_eq!(
            format!("{}\n", serde_json::to_string_pretty(&record).unwrap()),
            expected
        );
        let wire = serde_json::to_value(&record).unwrap();
        assert!(wire.get("payload").is_none());
        assert!(wire.get("evidence_refs").is_none());
        candidate.payload = CandidatePayload::RedactedSummary("Purged".into());
        assert_eq!(candidate.suppression_record(decision()).unwrap(), record);
    }

    #[test]
    fn suppression_builder_requires_rejection_key_and_identity_actor() {
        let mut candidate = candidate();
        for state in STATES {
            candidate.lifecycle_state = state;
            let result = candidate.suppression_record(decision());
            if state == CandidateLifecycleState::Rejected {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
            }
        }
        candidate.lifecycle_state = CandidateLifecycleState::Rejected;
        let mut non_identity = decision();
        non_identity.deciding_actor_identity_id = UbuId::new(ObjectType::Task);
        assert!(matches!(
            candidate.suppression_record(non_identity),
            Err(UbuError::WrongIdObjectType { .. })
        ));
        let mut record = candidate.suppression_record(decision()).unwrap();
        record.deciding_actor_identity_id = UbuId::new(ObjectType::Task);
        assert!(record.validate().is_err());
        assert!(
            serde_json::from_value::<SuppressionRecord>(serde_json::to_value(record).unwrap())
                .is_err()
        );
        for key in [None, Some(String::new())] {
            candidate.suppression_key = key;
            assert_eq!(
                candidate.suppression_record(decision()),
                Err(UbuError::InvalidSuppressionRecord {
                    field: "suppression_key"
                })
            );
        }
    }

    #[test]
    fn payload_variants_round_trip_without_redaction_type_confusion() {
        let original = include_str!(
            "../fixtures/placeholders/valid/core/advisory-candidate/redacted-payload.json"
        );
        let mut candidate: AdvisoryCandidate = serde_json::from_str(original).unwrap();
        assert!(matches!(
            &candidate.payload,
            CandidatePayload::RedactedSummary(_)
        ));
        assert_eq!(candidate.review_label, ReviewLabel::Redacted);
        assert_eq!(
            format!("{}\n", serde_json::to_string_pretty(&candidate).unwrap()),
            original
        );
        for payload in [
            CandidatePayload::Inline(json!("A summary-like inline string")),
            CandidatePayload::Inline(json!({"kind": "redacted_summary", "value": "nested"})),
            CandidatePayload::Inline(serde_json::Value::Null),
            CandidatePayload::RedactedSummary("private".into()),
        ] {
            candidate.payload = payload.clone();
            let bytes = serde_json::to_vec(&candidate).unwrap();
            let restored: AdvisoryCandidate = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(restored.payload, payload);
            assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
        }
        for wire in [
            json!({"kind":"redacted_summary","value":{}}),
            json!({"kind":"unknown","value":"private"}),
            json!({"kind":"inline","value":{},"extra":"private"}),
        ] {
            assert!(serde_json::from_value::<CandidatePayload>(wire).is_err());
        }
    }

    #[test]
    fn candidate_vocabularies_have_the_documented_wire_spellings() {
        for (kind, wire) in [
            (CandidateKind::Tag, "tag"),
            (CandidateKind::Dependency, "dependency"),
            (CandidateKind::Preference, "preference"),
            (CandidateKind::Decomposition, "decomposition"),
            (
                CandidateKind::ClarificationQuestion,
                "clarification_question",
            ),
        ] {
            assert_eq!(serde_json::to_value(kind).unwrap(), json!(wire));
            assert_eq!(
                serde_json::from_value::<CandidateKind>(json!(wire)).unwrap(),
                kind
            );
        }
        for (state, wire) in STATES.into_iter().zip([
            "proposed",
            "deferred",
            "resurfaced",
            "admitted",
            "rejected",
            "superseded",
            "archived",
        ]) {
            assert_eq!(serde_json::to_value(state).unwrap(), json!(wire));
            assert_eq!(
                serde_json::from_value::<CandidateLifecycleState>(json!(wire)).unwrap(),
                state
            );
        }
        for (trigger, wire) in [
            (
                ResurfaceTrigger::MateriallyNewEvidence,
                "materially_new_evidence",
            ),
            (ResurfaceTrigger::UserRequest, "user_request"),
            (
                ResurfaceTrigger::PolicyReviewInterval,
                "policy_review_interval",
            ),
            (
                ResurfaceTrigger::AcceptedChangeToTargetOrDependencies,
                "accepted_change_to_target_or_dependencies",
            ),
            (
                ResurfaceTrigger::ClarificationOrExternalReferenceArrival,
                "clarification_or_external_reference_arrival",
            ),
        ] {
            assert_eq!(serde_json::to_value(trigger).unwrap(), json!(wire));
            assert_eq!(
                serde_json::from_value::<ResurfaceTrigger>(json!(wire)).unwrap(),
                trigger
            );
        }
        for (policy, wire) in [
            (RetentionPolicy::Retain, "retain"),
            (RetentionPolicy::PurgePayload, "purge_payload"),
        ] {
            assert_eq!(serde_json::to_value(policy).unwrap(), json!(wire));
            assert_eq!(
                serde_json::from_value::<RetentionPolicy>(json!(wire)).unwrap(),
                policy
            );
        }
        for (policy, wire) in [
            (DisclosurePolicy::CompartmentOnly, "compartment_only"),
            (DisclosurePolicy::RedactedOnly, "redacted_only"),
        ] {
            assert_eq!(serde_json::to_value(policy).unwrap(), json!(wire));
            assert_eq!(
                serde_json::from_value::<DisclosurePolicy>(json!(wire)).unwrap(),
                policy
            );
        }
        assert!(serde_json::from_value::<CandidateLifecycleState>(json!("accepted")).is_err());
        assert!(serde_json::from_value::<DisclosurePolicy>(json!("public")).is_err());
        assert!(serde_json::from_value::<RetentionPolicy>(json!("forever")).is_err());
    }
}
