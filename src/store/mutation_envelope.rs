//! Canonical mutation metadata, independent of any store writer or call site.

use std::collections::BTreeMap;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{AuthoritySource, UbuError, UbuId, UbuTimestamp};

/// Stable operator-registered device identifier, not a canonical object id.
/// Only emptiness is forbidden; whitespace is not trimmed or normalized.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn parse(value: impl Into<String>) -> crate::Result<Self> {
        let value = value.into();
        if value.is_empty() {
            return Err(UbuError::EmptyDeviceId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for DeviceId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

/// A non-empty key scoped to its origin device.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    pub fn parse(value: impl Into<String>) -> crate::Result<Self> {
        let value = value.into();
        if value.is_empty() {
            return Err(UbuError::EmptyIdempotencyKey);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for IdempotencyKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

/// An object precondition, not an ordering clock or global version.
///
/// The schema's `^v[0-9]+$` is broader than `Version(u64)`: Rust rejects
/// overflow and leading zeros so every accepted version string round-trips
/// byte-identically. Canonical spellings are `v0`, `v1`, ..., or `absent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VersionRef {
    Version(u64),
    Absent,
}

impl Serialize for VersionRef {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Version(version) => serializer.serialize_str(&format!("v{version}")),
            Self::Absent => serializer.serialize_str("absent"),
        }
    }
}

impl<'de> Deserialize<'de> for VersionRef {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        if value == "absent" {
            return Ok(Self::Absent);
        }
        if let Some(digits) = value.strip_prefix('v') {
            if !digits.is_empty()
                && digits.bytes().all(|byte| byte.is_ascii_digit())
                && (digits == "0" || !digits.starts_with('0'))
            {
                if let Ok(version) = digits.parse::<u64>() {
                    return Ok(Self::Version(version));
                }
            }
        }
        Err(D::Error::custom(UbuError::InvalidVersionRef { value }))
    }
}

/// Non-authoritative provenance; unknown fields, including authority overrides,
/// are rejected. These optional labels impose no additional identity semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationEnvelope {
    pub idempotency_key: IdempotencyKey,
    pub observed_versions: BTreeMap<UbuId, VersionRef>,
    pub origin_device_id: DeviceId,
    pub actor_identity_id: UbuId,
    pub authority_source: AuthoritySource,
    pub created_time: UbuTimestamp,
    pub effective_time: UbuTimestamp,
    pub recorded_time: UbuTimestamp,
    // Policy values remain free strings, matching the requested schema and type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_policy_versions: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_context: Option<ExecutionContext>,
}
