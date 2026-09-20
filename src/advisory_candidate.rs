//! Advisory review-queue types, separate from admitted objects (UBU-D0274).

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::UbuError;

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
