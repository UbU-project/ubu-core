//! Canonical mutation metadata, independent of any store writer or call site.

use std::collections::BTreeMap;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

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
/// Schema and type share the canonical grammar: `v0`, `v1`, ..., or `absent`,
/// with no leading zeros. The type additionally bounds versions to `u64`;
/// every accepted version string round-trips byte-identically.
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

/// Domain inputs; origin device, idempotency key, and admission times are owned
/// by the issuer rather than by mutation call sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeRequest {
    pub observed_versions: BTreeMap<UbuId, VersionRef>,
    pub actor_identity_id: UbuId,
    pub authority_source: AuthoritySource,
    pub effective_time: UbuTimestamp,
    pub observed_policy_versions: Option<BTreeMap<String, String>>,
    pub execution_context: Option<ExecutionContext>,
}

/// Admission-owned seam for issuing a canonical mutation's envelope.
pub trait CausalityIssuer {
    fn issue(&self, request: EnvelopeRequest) -> crate::Result<MutationEnvelope>;
}

/// Phase 1b issuer with a unique, opaque UUIDv7 key minted per issuance.
///
/// Restart continuity does not depend on durable issuer state. Consumers must
/// not parse keys or depend on their ordering for correctness. The unchanged
/// `CausalityIssuer` trait lets Phase 2 supply HLC ticks and causal parents
/// behind the same method. Durable recording and replay handling remain future
/// admission integration responsibilities. A single injected clock reading
/// stamps both assembly and recording for this immediate local issuance seam;
/// the envelope type independently preserves all three timestamps.
pub struct LocalIssuer {
    device_id: DeviceId,
    clock: Box<dyn Fn() -> UbuTimestamp + Send + Sync>,
}

impl LocalIssuer {
    pub fn new(device_id: DeviceId) -> Self {
        Self::with_clock(device_id, UbuTimestamp::now_utc)
    }

    pub fn with_clock(
        device_id: DeviceId,
        clock: impl Fn() -> UbuTimestamp + Send + Sync + 'static,
    ) -> Self {
        Self {
            device_id,
            clock: Box::new(clock),
        }
    }
}

impl CausalityIssuer for LocalIssuer {
    fn issue(&self, request: EnvelopeRequest) -> crate::Result<MutationEnvelope> {
        let idempotency_key = IdempotencyKey::parse(Uuid::now_v7().simple().to_string())?;
        let now = (self.clock)();
        let envelope = MutationEnvelope {
            idempotency_key,
            observed_versions: request.observed_versions,
            origin_device_id: self.device_id.clone(),
            actor_identity_id: request.actor_identity_id,
            authority_source: request.authority_source,
            created_time: now,
            effective_time: request.effective_time,
            recorded_time: now,
            observed_policy_versions: request.observed_policy_versions,
            execution_context: request.execution_context,
        };
        envelope.validate()?;
        Ok(envelope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn envelope() -> MutationEnvelope {
        serde_json::from_str(include_str!(
            "../../fixtures/placeholders/valid/store/mutation-envelope/basic.json"
        ))
        .unwrap()
    }

    fn request() -> EnvelopeRequest {
        let envelope = envelope();
        EnvelopeRequest {
            observed_versions: envelope.observed_versions,
            actor_identity_id: envelope.actor_identity_id,
            authority_source: envelope.authority_source,
            effective_time: UbuTimestamp::parse("2026-06-10T02:00:00Z").unwrap(),
            observed_policy_versions: Some(BTreeMap::from([(
                "routing:default".into(),
                "v2".into(),
            )])),
            execution_context: Some(ExecutionContext {
                context_label: Some("advisory".into()),
                backend_id: None,
                provider_id: Some("local".into()),
            }),
        }
    }

    #[test]
    fn version_ref_round_trips_exact_strings() {
        for (wire, expected) in [
            ("\"v0\"", VersionRef::Version(0)),
            ("\"v17\"", VersionRef::Version(17)),
            ("\"absent\"", VersionRef::Absent),
            ("\"v18446744073709551615\"", VersionRef::Version(u64::MAX)),
        ] {
            let version: VersionRef = serde_json::from_str(wire).unwrap();
            assert_eq!(version, expected);
            assert_eq!(serde_json::to_string(&version).unwrap(), wire);
        }
    }

    #[test]
    fn version_ref_rejects_other_spellings_and_overflow() {
        for wire in [
            "17",
            "\"17\"",
            "\"v\"",
            "\"V17\"",
            "\"v-1\"",
            "\"deleted\"",
            "\"v+1\"",
            "\"v01\"",
            "\"v00\"",
            "\"v18446744073709551616\"",
            "\"v١\"",
            "\"v17\\n\"",
            "null",
            "{}",
        ] {
            assert!(serde_json::from_str::<VersionRef>(wire).is_err(), "{wire}");
        }
    }

    #[test]
    fn version_ref_rejects_leading_zeros_and_accepts_zero() {
        for value in ["v007", "v00", "v01"] {
            assert!(serde_json::from_value::<VersionRef>(json!(value)).is_err());
        }
        assert_eq!(
            serde_json::from_value::<VersionRef>(json!("v0")).unwrap(),
            VersionRef::Version(0)
        );
    }

    #[test]
    fn version_ref_rejects_canonical_value_beyond_u64_range() {
        // Deliberately not a schema fixture: the schema accepts this canonical
        // 20-digit spelling by design; the consuming Rust type bounds its range.
        let value = "v99999999999999999999";
        let error = serde_json::from_value::<VersionRef>(json!(value)).unwrap_err();
        assert_eq!(
            error.to_string(),
            UbuError::InvalidVersionRef {
                value: value.to_owned(),
            }
            .to_string()
        );
    }

    #[test]
    fn validate_requires_identity_actor() {
        let mut value = envelope();
        value.actor_identity_id = value.observed_versions.keys().next().unwrap().clone();
        assert!(matches!(
            value.validate(),
            Err(UbuError::WrongIdObjectType { .. })
        ));
    }

    #[test]
    fn validate_rejects_empty_device_and_key() {
        let mut value = envelope();
        value.origin_device_id = DeviceId(String::new());
        assert_eq!(value.validate(), Err(UbuError::EmptyDeviceId));
        let mut value = envelope();
        value.idempotency_key = IdempotencyKey(String::new());
        assert_eq!(value.validate(), Err(UbuError::EmptyIdempotencyKey));
        assert_eq!(DeviceId::parse(""), Err(UbuError::EmptyDeviceId));
        assert_eq!(
            IdempotencyKey::parse(""),
            Err(UbuError::EmptyIdempotencyKey)
        );
        assert!(serde_json::from_str::<DeviceId>("\"\"").is_err());
        assert!(serde_json::from_str::<IdempotencyKey>("\"\"").is_err());
        assert_eq!(DeviceId::parse(" ").unwrap().as_str(), " ");
        assert_eq!(IdempotencyKey::parse(" ").unwrap().as_str(), " ");
    }

    #[test]
    fn invalid_observed_id_is_rejected_before_validate() {
        // UbuId's private, validated representation prevents constructing an
        // invalid map key. Exercise that invariant at the deserialization edge.
        let mut value = serde_json::to_value(envelope()).unwrap();
        value["observed_versions"] = json!({"not-a-ubu-id": "v17"});
        assert!(serde_json::from_value::<MutationEnvelope>(value).is_err());
        let mut value = envelope();
        value.observed_versions.clear();
        value.validate().unwrap();
    }

    #[test]
    fn policy_versions_are_required_only_when_requested() {
        let mut value = envelope();
        value.validate().unwrap();
        assert_eq!(
            value.require_policy_versions(),
            Err(UbuError::MissingObservedPolicyVersions)
        );
        value.observed_policy_versions = Some(BTreeMap::new());
        assert_eq!(
            value.require_policy_versions(),
            Err(UbuError::MissingObservedPolicyVersions)
        );
        value.observed_policy_versions =
            Some(BTreeMap::from([("routing:default".into(), "v2".into())]));
        value.require_policy_versions().unwrap();
    }

    #[test]
    fn overnight_times_remain_independent_through_round_trip() {
        let value: MutationEnvelope = serde_json::from_str(include_str!(
            "../../fixtures/placeholders/valid/store/mutation-envelope/overnight-advisory.json"
        ))
        .unwrap();
        value.validate().unwrap();
        assert_eq!(value.created_time.to_string(), "2026-06-10T02:00:00Z");
        assert_eq!(value.effective_time.to_string(), "2026-06-10T02:00:00Z");
        assert_eq!(value.recorded_time.to_string(), "2026-06-10T09:00:00Z");
        assert!(value.created_time < value.recorded_time);
        let restored: MutationEnvelope =
            serde_json::from_slice(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(restored, value);
    }

    #[test]
    fn duplicate_key_is_scoped_to_device_and_idempotency_key() {
        let issuer = LocalIssuer::new(DeviceId::parse("local").unwrap());
        let first = (issuer.issue(request()).unwrap(), json!({"title": "first"}));
        let second = (issuer.issue(request()).unwrap(), json!({"title": "second"}));
        assert_ne!(
            canonical_payload_bytes(&first.1),
            canonical_payload_bytes(&second.1)
        );
        assert_ne!(first.0.mutation_key(), second.0.mutation_key());
        let mut replay = second.0;
        replay.idempotency_key = first.0.idempotency_key.clone();
        assert_eq!(first.0.mutation_key(), replay.mutation_key());
        let keys = std::collections::HashSet::from([first.0.mutation_key(), replay.mutation_key()]);
        assert_eq!(keys.len(), 1);
        replay.origin_device_id = DeviceId::parse("other").unwrap();
        assert_ne!(first.0.mutation_key(), replay.mutation_key());
    }

    #[test]
    fn canonical_payload_sorts_recursively_and_preserves_values() {
        let first: Value = serde_json::from_str(r#"{"z":[{"b":2,"a":1}],"a":"line\n\""}"#).unwrap();
        let reordered: Value =
            serde_json::from_str(r#"{"a":"line\n\"","z":[{"a":1,"b":2}]}"#).unwrap();
        assert_eq!(
            canonical_payload_bytes(&first),
            canonical_payload_bytes(&reordered)
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&canonical_payload_bytes(&first)).unwrap(),
            first
        );
        for replacement in [
            json!(3),
            json!("2"),
            json!(null),
            json!(false),
            json!([]),
            json!({}),
        ] {
            let mut changed = first.clone();
            changed["z"][0]["b"] = replacement;
            assert_ne!(
                canonical_payload_bytes(&first),
                canonical_payload_bytes(&changed)
            );
        }
        assert_ne!(
            canonical_payload_bytes(&json!([1, 2])),
            canonical_payload_bytes(&json!([2, 1]))
        );
    }

    #[test]
    fn issuer_stamps_device_clock_and_unique_keys() {
        let now = UbuTimestamp::parse("2026-06-10T09:00:00Z").unwrap();
        let issuer =
            LocalIssuer::with_clock(DeviceId::parse("registered-device").unwrap(), move || now);
        let issuer: &dyn CausalityIssuer = &issuer;
        let mut keys = std::collections::HashSet::new();
        for _ in 0..1024 {
            let request = request();
            let value = issuer.issue(request.clone()).unwrap();
            assert_eq!(value.origin_device_id.as_str(), "registered-device");
            assert_eq!(value.created_time, now);
            assert_eq!(value.recorded_time, now);
            assert_eq!(value.effective_time, request.effective_time);
            assert_eq!(value.observed_versions, request.observed_versions);
            assert_eq!(value.actor_identity_id, request.actor_identity_id);
            assert_eq!(value.authority_source, request.authority_source);
            assert_eq!(
                value.observed_policy_versions,
                request.observed_policy_versions
            );
            assert_eq!(value.execution_context, request.execution_context);
            assert!(keys.insert(value.idempotency_key));
        }
    }

    #[test]
    fn recreated_issuer_does_not_repeat_keys_for_same_device() {
        let device_id = DeviceId::parse("local").unwrap();
        let now = UbuTimestamp::parse("2026-06-10T09:00:00Z").unwrap();
        let first = {
            let issuer = LocalIssuer::with_clock(device_id.clone(), move || now);
            issuer.issue(request()).unwrap()
        };
        let restarted = LocalIssuer::with_clock(device_id, move || now);
        let second = restarted.issue(request()).unwrap();
        assert_eq!(first.origin_device_id, second.origin_device_id);
        assert_eq!(first.created_time, second.created_time);
        assert_ne!(first.idempotency_key, second.idempotency_key);
        assert_ne!(first.mutation_key(), second.mutation_key());
    }

    #[test]
    fn issuer_rejects_invalid_request() {
        let issuer = LocalIssuer::new(DeviceId::parse("local").unwrap());
        let mut request = request();
        request.actor_identity_id = request.observed_versions.keys().next().unwrap().clone();
        assert!(matches!(
            issuer.issue(request),
            Err(UbuError::WrongIdObjectType { .. })
        ));
    }

    #[test]
    fn execution_context_cannot_restate_authority() {
        for field in ["origin_device_id", "actor_identity_id", "authority_source"] {
            let mut value = serde_json::to_value(envelope()).unwrap();
            value["execution_context"] = json!({field: "override"});
            assert!(serde_json::from_value::<MutationEnvelope>(value).is_err());
        }
        let mut value = serde_json::to_value(envelope()).unwrap();
        value["unknown"] = json!("extra");
        assert!(serde_json::from_value::<MutationEnvelope>(value).is_err());
    }
}
