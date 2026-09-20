//! Device registration material and registry semantics, without persistence or I/O.

use std::collections::BTreeSet;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

pub use crate::store::mutation_envelope::DeviceId;
use crate::{CompartmentLabel, ObjectType, UbuError, UbuId, UbuTimestamp};

/// Exactly one opaque, non-empty Zone identifier; this does not define a Zone object.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ZoneId(String);

impl ZoneId {
    pub fn parse(value: impl Into<String>) -> crate::Result<Self> {
        let value = value.into();
        if value.is_empty() {
            return Err(UbuError::EmptyZoneId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ZoneId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

/// Enclave forms from DESIGN §23.1. A kind alone does not confer registration
/// or authority on a browser context or worker process.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    OsUserProfile,
    Container,
    VirtualMachine,
    SecureEnclave,
    BrowserProfile,
    BrowserSession,
    WorkerProcess,
    WorkerRuntime,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustState {
    Registered,
    Revoked,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    /// Phase 1b local-only registration, never synced.
    LocalOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceRegistration {
    pub device_id: DeviceId,
    pub label: String,
    pub kind: DeviceKind,
    pub registered_at: UbuTimestamp,
    #[serde(deserialize_with = "deserialize_identity")]
    pub registered_identity_id: UbuId,
    pub trust_state: TrustState,
    pub sync_state: SyncState,
    pub zone_id: ZoneId,
    #[serde(deserialize_with = "deserialize_unique_set")]
    pub capability_profile: BTreeSet<String>,
    /// Allowlist only; every unlisted label is denied regardless of registry size.
    #[serde(deserialize_with = "deserialize_unique_set")]
    pub compartment_access: BTreeSet<CompartmentLabel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<UbuTimestamp>,
}

fn deserialize_identity<'de, D: Deserializer<'de>>(deserializer: D) -> Result<UbuId, D::Error> {
    let id = UbuId::deserialize(deserializer)?;
    id.require_object_type(ObjectType::Identity)
        .map_err(D::Error::custom)?;
    Ok(id)
}

// Preserve the schema's uniqueItems constraint instead of silently deduplicating.
fn deserialize_unique_set<'de, D, T>(deserializer: D) -> Result<BTreeSet<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Ord,
{
    let mut set = BTreeSet::new();
    for item in Vec::<T>::deserialize(deserializer)? {
        if !set.insert(item) {
            return Err(D::Error::custom("duplicate entry in registration set"));
        }
    }
    Ok(set)
}

impl DeviceRegistration {
    /// Validate registration inputs. Non-empty strings are not trimmed;
    /// all three timestamps remain independent of registration validation.
    pub fn validate(&self) -> crate::Result<()> {
        self.registered_identity_id
            .require_object_type(ObjectType::Identity)?;
        if self.label.is_empty() {
            return Err(UbuError::EmptyDeviceLabel);
        }
        ZoneId::parse(self.zone_id.as_str())?;
        if self.capability_profile.iter().any(String::is_empty) {
            return Err(UbuError::EmptyDeviceCapability);
        }
        Ok(())
    }

    /// Test only explicit allowlist membership; registry cardinality grants nothing.
    pub fn may_access(&self, label: &CompartmentLabel) -> bool {
        self.compartment_access.contains(label)
    }

    /// The admission path must perform this check before accepting an origin.
    /// Phase 1b's single registration is not exempt. This does not grant access
    /// to a Compartment; its allowlist must be checked independently.
    pub fn may_originate_mutations(&self) -> bool {
        matches!(self.trust_state, TrustState::Registered)
    }
}

impl DeviceId {
    /// Assign a fresh identifier at registration: `dev_` plus a UUIDv7.
    /// Never derive Device identity from hardware, installation identity,
    /// database contents, or integration credentials. Continuity requires
    /// preserving/restoring registration material; losing it means a new Device.
    ///
    /// Minted ids use this shape, while the existing `parse` deliberately accepts
    /// any non-empty operator-supplied registration identifier.
    pub fn generate() -> Self {
        Self::parse(format!("dev_{}", Uuid::now_v7().simple()))
            .expect("generated Device identifier is non-empty")
    }
}
