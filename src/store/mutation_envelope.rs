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

impl MutationEnvelope {
    /// Check envelope invariants without imposing ordering on its three times.
    pub fn validate(&self) -> crate::Result<()> {
        self.actor_identity_id
            .require_object_type(crate::ObjectType::Identity)?;
        DeviceId::parse(self.origin_device_id.as_str())?;
        IdempotencyKey::parse(self.idempotency_key.as_str())?;
        for id in self.observed_versions.keys() {
            UbuId::parse(id.as_str())?;
        }
        Ok(())
    }

    /// Policy-dependent call sites MUST call this in addition to `validate`.
    /// Neither the schema nor this envelope can infer whether the mutation
    /// relies on Compartment, Zone, projection, routing, or other policy state.
    pub fn require_policy_versions(&self) -> crate::Result<()> {
        match &self.observed_policy_versions {
            Some(versions) if !versions.is_empty() => Ok(()),
            _ => Err(UbuError::MissingObservedPolicyVersions),
        }
    }

    pub fn mutation_key(&self) -> MutationKey {
        MutationKey {
            origin_device_id: self.origin_device_id.clone(),
            idempotency_key: self.idempotency_key.clone(),
        }
    }
}

/// Duplicate-detection key; replay handling belongs to the admission writer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MutationKey {
    pub origin_device_id: DeviceId,
    pub idempotency_key: IdempotencyKey,
}

/// Compact UTF-8 JSON with recursively sorted object keys and preserved array
/// order and values. This defines payload equality without a hashing dependency.
pub fn canonical_payload_bytes(payload: &serde_json::Value) -> Vec<u8> {
    fn write(value: &serde_json::Value, bytes: &mut Vec<u8>) {
        match value {
            serde_json::Value::Object(object) => {
                bytes.push(b'{');
                let sorted: BTreeMap<_, _> = object.iter().collect();
                for (index, (key, value)) in sorted.into_iter().enumerate() {
                    if index != 0 {
                        bytes.push(b',');
                    }
                    serde_json::to_writer(&mut *bytes, key).expect("JSON key serializes");
                    bytes.push(b':');
                    write(value, bytes);
                }
                bytes.push(b'}');
            }
            serde_json::Value::Array(array) => {
                bytes.push(b'[');
                for (index, value) in array.iter().enumerate() {
                    if index != 0 {
                        bytes.push(b',');
                    }
                    write(value, bytes);
                }
                bytes.push(b']');
            }
            _ => serde_json::to_writer(bytes, value).expect("JSON value serializes"),
        }
    }

    let mut bytes = Vec::new();
    write(payload, &mut bytes);
    bytes
}
