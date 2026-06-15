use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use ubu_core::core::{MootReasonCode, Task, TaskStatus};
use ubu_core::validation::validate_task_lifecycle;
use ubu_core::UbuError;

#[test]
fn valid_moot_task_fixture_passes_lifecycle_validation() {
    let original = fixture_json("valid/core/task/moot.json");
    let task: Task = serde_json::from_value(original.clone()).expect("task fixture deserializes");

    assert_eq!(task.status, TaskStatus::Moot);
    assert_eq!(task.moot_reason_code, Some(MootReasonCode::Duplicate));
    assert_eq!(validate_task_lifecycle(&task), Ok(()));
    assert_eq!(
        serde_json::to_value(task).expect("serialize task"),
        original
    );
}

#[test]
fn moot_reason_codes_round_trip_all_wire_values() {
    let cases = [
        ("externally_satisfied", MootReasonCode::ExternallySatisfied),
        ("superseded", MootReasonCode::Superseded),
        ("delegated", MootReasonCode::Delegated),
        ("no_longer_relevant", MootReasonCode::NoLongerRelevant),
        (
            "invalidated_by_universe_change",
            MootReasonCode::InvalidatedByUniverseChange,
        ),
        (
            "replaced_by_new_plan_structure",
            MootReasonCode::ReplacedByNewPlanStructure,
        ),
        ("user_declared_moot", MootReasonCode::UserDeclaredMoot),
        ("automation_obsolete", MootReasonCode::AutomationObsolete),
        ("duplicate", MootReasonCode::Duplicate),
    ];

    for (wire, expected) in cases {
        let parsed: MootReasonCode = serde_json::from_str(&format!("\"{wire}\"")).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(
            serde_json::to_value(parsed).expect("serialize moot reason code"),
            serde_json::json!(wire)
        );
    }
}

#[test]
fn moot_task_missing_reason_fails_lifecycle_validation() {
    let task = task_fixture("invalid/core/task/moot-without-reason.json");

    assert_eq!(task.status, TaskStatus::Moot);
    assert_eq!(task.moot_reason_code, None);
    assert_eq!(
        validate_task_lifecycle(&task),
        Err(UbuError::MissingMootReasonCode)
    );
}

#[test]
fn active_task_with_moot_reason_fails_lifecycle_validation() {
    let task = task_fixture("invalid/core/task/active-with-moot-reason.json");

    assert_eq!(task.status, TaskStatus::Active);
    assert_eq!(task.moot_reason_code, Some(MootReasonCode::Duplicate));
    assert_eq!(
        validate_task_lifecycle(&task),
        Err(UbuError::UnexpectedMootReasonCode { status: "active" })
    );
}

#[test]
fn persisted_noncanonical_status_values_fail_deserialization() {
    for status in ["ready", "in_progress", "proposed", "blocked", "canceled"] {
        let json = format!(
            r#"{{
  "id": "task_018f3c8e9b2a7c4d8f1e2a3b4c5d6e7f",
  "title": "Noncanonical status",
  "status": "{status}",
  "provenance": {{
    "created_at": "2026-06-10T14:30:00Z",
    "authority_source": "user"
  }}
}}"#
        );

        assert!(
            serde_json::from_str::<Task>(&json).is_err(),
            "{status} should not deserialize as a persisted Task.status"
        );
    }
}

fn task_fixture(relative: &str) -> Task {
    serde_json::from_value(fixture_json(relative)).expect("task fixture should deserialize")
}

fn fixture_json(relative: &str) -> Value {
    let path = PathBuf::from(env!("UBU_SCHEMAS_FIXTURES")).join(relative);
    let json = fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("failed to read fixture {}: {err}", path.display());
    });
    serde_json::from_str(&json).expect("fixture should be valid json")
}
