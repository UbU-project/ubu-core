//! Device registration material and registry semantics, without persistence or I/O.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

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
    /// registration and last-seen timestamps have no imposed ordering.
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

/// Pure in-memory registry. Deliberately exposes no singleton accessor:
/// UBU-D0257 says a registry of one is not proof of global authority.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceRegistry {
    registrations: BTreeMap<DeviceId, DeviceRegistration>,
}

impl DeviceRegistry {
    /// Build a registry from validated records, rejecting duplicate identifiers.
    pub fn new(registrations: impl IntoIterator<Item = DeviceRegistration>) -> crate::Result<Self> {
        let mut entries = BTreeMap::new();
        for registration in registrations {
            registration.validate()?;
            match entries.entry(registration.device_id.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(registration);
                }
                Entry::Occupied(entry) => {
                    return Err(UbuError::DuplicateDeviceId {
                        device_id: entry.key().as_str().to_owned(),
                    });
                }
            }
        }
        Ok(Self {
            registrations: entries,
        })
    }

    pub fn get(&self, device_id: &DeviceId) -> Option<&DeviceRegistration> {
        self.registrations.get(device_id)
    }

    pub fn len(&self) -> usize {
        self.registrations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.registrations.is_empty()
    }

    /// Iterate all registrations in stable DeviceId order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &DeviceRegistration> {
        self.registrations.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn registration() -> DeviceRegistration {
        serde_json::from_str(include_str!(
            "../fixtures/placeholders/valid/core/device-registration/phase1b-single-device.json"
        ))
        .unwrap()
    }

    fn label(value: &str) -> CompartmentLabel {
        CompartmentLabel::parse(value).unwrap()
    }

    #[test]
    fn validate_rejects_non_identity_association() {
        let mut value = registration();
        value.registered_identity_id =
            UbuId::parse("task_018f3c8e9b2a7c4d8f1e2a3b4c5d6e7f").unwrap();
        assert!(matches!(
            value.validate(),
            Err(UbuError::WrongIdObjectType { .. })
        ));
    }

    #[test]
    fn validate_rejects_empty_label_zone_and_capability() {
        let mut value = registration();
        value.label.clear();
        assert_eq!(value.validate(), Err(UbuError::EmptyDeviceLabel));
        let mut value = registration();
        value.zone_id = ZoneId(String::new());
        assert_eq!(value.validate(), Err(UbuError::EmptyZoneId));
        assert_eq!(ZoneId::parse(""), Err(UbuError::EmptyZoneId));
        assert!(serde_json::from_str::<ZoneId>("\"\"").is_err());
        let mut value = registration();
        value.capability_profile.insert(String::new());
        assert_eq!(value.validate(), Err(UbuError::EmptyDeviceCapability));
        value.capability_profile.clear();
        value.validate().unwrap();
    }

    #[test]
    fn compartment_access_defaults_to_deny() {
        let mut value = registration();
        assert!(value.may_access(&label("Personal")));
        for absent in ["Private", "Personal/Secret", "personal"] {
            assert!(!value.may_access(&label(absent)));
        }
        value.compartment_access.clear();
        for name in ["Personal", "Work", "Private", "Personal/Secret", "personal"] {
            assert!(!value.may_access(&label(name)));
        }
    }

    #[test]
    fn revoked_device_cannot_originate_mutations() {
        let mut value = registration();
        assert!(value.may_originate_mutations());
        value.trust_state = TrustState::Revoked;
        assert!(!value.may_originate_mutations());
        // Access summary membership is independent of permission to originate.
        assert!(value.may_access(&label("Personal")));
    }

    #[test]
    fn registry_cardinality_does_not_confer_authority() {
        let first = registration();
        let mut second = first.clone();
        second.device_id = DeviceId::parse("other-registered-device").unwrap();
        second.compartment_access = BTreeSet::from([label("Private")]);
        let one = DeviceRegistry::new([first.clone()]).unwrap();
        let two = DeviceRegistry::new([second.clone(), first.clone()]).unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(two.len(), 2);
        assert!(!one.is_empty());
        assert!(!two.is_empty());
        assert_eq!(one.get(&first.device_id), two.get(&first.device_id));
        let unknown = DeviceId::parse("unregistered-device").unwrap();
        for registry in [&one, &two] {
            assert_eq!(registry.get(&unknown), None);
            assert_eq!(registry.iter().len(), registry.len());
            for entry in registry.iter() {
                assert_eq!(registry.get(&entry.device_id), Some(entry));
            }
            let shared: Vec<_> = registry
                .iter()
                .filter(|entry| entry.device_id == first.device_id)
                .collect();
            assert_eq!(shared, vec![&first]);
            let entry = registry.get(&first.device_id).unwrap();
            for name in ["Personal", "Work", "Private", "Personal/Secret"] {
                let compartment = label(name);
                assert_eq!(
                    entry.may_access(&compartment),
                    first.compartment_access.contains(&compartment)
                );
            }
        }
        assert_eq!(one.get(&second.device_id), None);
        assert_eq!(two.get(&second.device_id), Some(&second));
        assert!(!one
            .get(&first.device_id)
            .unwrap()
            .may_access(&label("Private")));
        let forward = DeviceRegistry::new([first, second]).unwrap();
        assert_eq!(
            forward.iter().collect::<Vec<_>>(),
            two.iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn empty_registry_has_no_implicit_device() {
        let registry = DeviceRegistry::new([]).unwrap();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert_eq!(registry.iter().next(), None);
        assert_eq!(registry.get(&registration().device_id), None);
    }

    #[test]
    fn registry_rejects_duplicate_ids() {
        let first = registration();
        for change_label in [false, true] {
            let mut duplicate = first.clone();
            if change_label {
                duplicate.label = "Different registration contents".into();
            }
            assert_eq!(
                DeviceRegistry::new([first.clone(), duplicate]),
                Err(UbuError::DuplicateDeviceId {
                    device_id: first.device_id.as_str().to_owned(),
                })
            );
        }
    }

    #[test]
    fn registry_rejects_invalid_registration() {
        let mut value = registration();
        value.label.clear();
        assert_eq!(
            DeviceRegistry::new([value]),
            Err(UbuError::EmptyDeviceLabel)
        );
    }

    #[test]
    fn generated_device_ids_are_distinct_and_operator_ids_still_parse() {
        let mut ids = BTreeSet::new();
        for _ in 0..256 {
            let id = DeviceId::generate();
            let suffix = id.as_str().strip_prefix("dev_").unwrap();
            assert_eq!(suffix.len(), 32);
            let uuid = Uuid::parse_str(suffix).unwrap();
            assert_eq!(uuid.get_version_num(), 7);
            assert_eq!(uuid.simple().to_string(), suffix);
            assert_eq!(DeviceId::parse(id.as_str()).unwrap(), id);
            assert!(ids.insert(id));
        }
        let supplied = "operator-restored-registration";
        assert_eq!(DeviceId::parse(supplied).unwrap().as_str(), supplied);
    }

    #[test]
    fn registration_sets_serialize_stably_in_sorted_order() {
        let expected = registration();
        let mut reversed = expected.clone();
        reversed.capability_profile.clear();
        reversed.compartment_access.clear();
        for capability in expected.capability_profile.iter().rev() {
            reversed.capability_profile.insert(capability.clone());
        }
        for compartment in expected.compartment_access.iter().rev() {
            reversed.compartment_access.insert(compartment.clone());
        }
        let bytes = serde_json::to_vec(&expected).unwrap();
        assert_eq!(serde_json::to_vec(&reversed).unwrap(), bytes);
        let restored: DeviceRegistration = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored, expected);
        assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
        let value = serde_json::to_value(restored).unwrap();
        assert_eq!(
            value["capability_profile"],
            json!(["admission", "local_advisory"])
        );
        assert_eq!(value["compartment_access"], json!(["Personal", "Work"]));
    }

    #[test]
    fn registration_deserialization_rejects_duplicate_set_entries() {
        for (field, duplicate) in [
            ("capability_profile", "admission"),
            ("compartment_access", "Personal"),
        ] {
            let mut value = serde_json::to_value(registration()).unwrap();
            value[field] = json!([duplicate, duplicate]);
            assert!(serde_json::from_value::<DeviceRegistration>(value).is_err());
        }
    }

    #[test]
    fn registration_vocabularies_match_the_schema() {
        for (kind, spelling) in [
            (DeviceKind::OsUserProfile, "os_user_profile"),
            (DeviceKind::Container, "container"),
            (DeviceKind::VirtualMachine, "virtual_machine"),
            (DeviceKind::SecureEnclave, "secure_enclave"),
            (DeviceKind::BrowserProfile, "browser_profile"),
            (DeviceKind::BrowserSession, "browser_session"),
            (DeviceKind::WorkerProcess, "worker_process"),
            (DeviceKind::WorkerRuntime, "worker_runtime"),
        ] {
            assert_eq!(serde_json::to_value(kind).unwrap(), json!(spelling));
            assert_eq!(
                serde_json::from_value::<DeviceKind>(json!(spelling)).unwrap(),
                kind
            );
        }
        for (trust, spelling) in [
            (TrustState::Registered, "registered"),
            (TrustState::Revoked, "revoked"),
        ] {
            assert_eq!(serde_json::to_value(trust).unwrap(), json!(spelling));
            assert_eq!(
                serde_json::from_value::<TrustState>(json!(spelling)).unwrap(),
                trust
            );
        }
        assert_eq!(
            serde_json::to_value(SyncState::LocalOnly).unwrap(),
            json!("local_only")
        );
        assert!(serde_json::from_value::<DeviceKind>(json!("external_calendar")).is_err());
        assert!(serde_json::from_value::<TrustState>(json!("trusted")).is_err());
        assert!(serde_json::from_value::<SyncState>(json!("synced")).is_err());
    }
}
