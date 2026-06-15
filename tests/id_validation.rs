use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use ubu_core::id_registry::{
    object_type_for_prefix, object_type_from_id, prefix_entries, ObjectType,
};
use ubu_core::ids::UbuId;

const VALID_TASK: &str = "task_018f3c8e9b2a7c4d8f1e2a3b4c5d6e7f";

#[test]
fn accepts_valid_prefixed_ids() {
    let id = UbuId::parse(VALID_TASK).expect("valid task id");
    assert_eq!(id.as_str(), VALID_TASK);
    assert_eq!(id.object_type(), ObjectType::Task);
    assert_eq!(object_type_for_prefix("task_"), Some(ObjectType::Task));
}

#[test]
fn rejects_invalid_prefixes() {
    assert!(UbuId::parse("bad_018f3c8e9b2a7c4d8f1e2a3b4c5d6e7f").is_err());
}

#[test]
fn rejects_hyphenated_uppercase_and_wrong_length_suffixes() {
    assert!(UbuId::parse("task_018f3c8e-9b2a-7c4d-8f1e-2a3b4c5d6e7f").is_err());
    assert!(UbuId::parse("task_018F3C8E9B2A7C4D8F1E2A3B4C5D6E7F").is_err());
    assert!(UbuId::parse("task_018f3c8e9b2a7c4d8f1e2a3b4c5d6e").is_err());
}

#[test]
fn generates_uuid_v7_prefixed_ids() {
    let id = UbuId::new(ObjectType::Plan);
    assert_eq!(id.object_type(), ObjectType::Plan);
    assert!(id.as_str().starts_with("plan_"));
    assert_eq!(id.as_str().len(), "plan_".len() + 32);
}

#[test]
fn accepts_new_object_type_prefix_fixtures() {
    for (fixture_path, expected_prefix, expected_type) in new_object_type_cases() {
        let id = valid_id_fixture(fixture_path);

        assert_eq!(id.object_type(), expected_type);
        assert_eq!(id.prefix(), expected_prefix);
        assert_eq!(object_type_for_prefix(expected_prefix), Some(expected_type));
        assert_eq!(object_type_from_id(id.as_str()), Some(expected_type));
        id.require_object_type(expected_type)
            .expect("fixture id has expected object type");
    }
}

#[test]
fn rejects_wrong_prefix_fixtures_for_new_object_types() {
    for (fixture_path, _, expected_type) in new_object_type_cases() {
        let wrong_prefix_fixture = fixture_path
            .replace("valid/common/id/", "invalid/common/id/")
            .replace("-id.json", "-wrong-prefix.json");
        let raw = string_fixture(&wrong_prefix_fixture);

        assert!(
            UbuId::parse(&raw).is_err(),
            "{wrong_prefix_fixture} should fail prefix validation"
        );

        let valid_id = valid_id_fixture(fixture_path);
        let wrong_expected = match expected_type {
            ObjectType::Task => ObjectType::Objective,
            _ => ObjectType::Task,
        };
        assert!(
            valid_id.require_object_type(wrong_expected).is_err(),
            "{fixture_path} should fail a mismatched require_object_type check"
        );
    }
}

#[test]
fn object_type_registry_agrees_with_schema_registry() {
    let schema = schema_registry();
    let schema_prefixes = schema_enum(&schema, "allowed_prefix");
    let schema_object_types = schema_enum(&schema, "object_type");
    let code_prefixes: Vec<_> = prefix_entries()
        .iter()
        .map(|entry| entry.prefix.to_owned())
        .collect();
    let code_object_types: Vec<_> = prefix_entries()
        .iter()
        .map(|entry| entry.object_type.as_str().to_owned())
        .collect();

    assert_eq!(code_prefixes, schema_prefixes);
    assert_eq!(code_object_types, schema_object_types);

    for entry in prefix_entries() {
        let id = format!("{}018f3c8e9b2a7c4d8f1e2a3b4c5d6e7f", entry.prefix);
        let parsed = UbuId::parse(&id).expect("schema registry id parses");

        assert_eq!(parsed.object_type(), entry.object_type);
        assert_eq!(object_type_from_id(&id), Some(entry.object_type));
    }
}

#[test]
fn rejects_wrong_uuid_v7_version_and_variant_bits() {
    assert!(UbuId::parse("task_018f3c8e9b2a6c4d8f1e2a3b4c5d6e7f").is_err());
    assert!(UbuId::parse("task_018f3c8e9b2a7c4d7f1e2a3b4c5d6e7f").is_err());
}

fn new_object_type_cases() -> [(&'static str, &'static str, ObjectType); 6] {
    [
        (
            "valid/common/id/preference-id.json",
            "pref_",
            ObjectType::Preference,
        ),
        (
            "valid/common/id/container-id.json",
            "container_",
            ObjectType::Container,
        ),
        (
            "valid/common/id/universe-state-id.json",
            "ustate_",
            ObjectType::UniverseState,
        ),
        (
            "valid/common/id/identity-id.json",
            "identity_",
            ObjectType::Identity,
        ),
        (
            "valid/common/id/relationship-id.json",
            "rel_",
            ObjectType::Relationship,
        ),
        (
            "valid/common/id/external-event-id.json",
            "xevent_",
            ObjectType::ExternalEvent,
        ),
    ]
}

fn valid_id_fixture(relative: &str) -> UbuId {
    UbuId::parse(string_fixture(relative)).expect("valid id fixture parses")
}

fn string_fixture(relative: &str) -> String {
    let path = PathBuf::from(env!("UBU_SCHEMAS_FIXTURES")).join(relative);
    let json = fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("failed to read fixture {}: {err}", path.display());
    });
    serde_json::from_str(&json).expect("fixture should be a json string")
}

fn schema_registry() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("schemas-ref")
        .join("schemas")
        .join("common")
        .join("id-registry.schema.json");
    let json = fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("failed to read schema {}: {err}", path.display());
    });
    serde_json::from_str(&json).expect("schema should be valid json")
}

fn schema_enum(schema: &Value, name: &str) -> Vec<String> {
    schema["$defs"][name]["enum"]
        .as_array()
        .unwrap_or_else(|| panic!("schema enum {name} should be present"))
        .iter()
        .map(|value| {
            value
                .as_str()
                .unwrap_or_else(|| panic!("schema enum {name} should contain strings"))
                .to_owned()
        })
        .collect()
}
