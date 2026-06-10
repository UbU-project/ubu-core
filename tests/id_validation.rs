use ubu_core::id_registry::{object_type_for_prefix, ObjectType};
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
