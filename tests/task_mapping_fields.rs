use serde_json::{json, Value};
use ubu_core::core::{validate_category_tag, StaticWindow, Task, TaskDurationEstimate, TaskStatus};
use ubu_core::{UbuError, UbuTimestamp};

fn base() -> Task {
    serde_json::from_str(include_str!(
        "../fixtures/placeholders/valid/core/task/basic.json"
    ))
    .unwrap()
}

fn window(start: &str, end: &str) -> StaticWindow {
    StaticWindow {
        start: UbuTimestamp::parse(start).unwrap(),
        end: UbuTimestamp::parse(end).unwrap(),
    }
}

#[test]
fn constructor_defaults_every_optional_field_without_validating() {
    let source = base();
    let task = Task::new(
        source.id.clone(),
        source.title.clone(),
        source.status,
        source.provenance.clone(),
    );
    assert_eq!(task.id, source.id);
    assert_eq!(task.title, source.title);
    assert_eq!(task.status, source.status);
    assert_eq!(task.provenance, source.provenance);
    assert!(task.description.is_none());
    assert!(task.moot_reason_code.is_none());
    assert!(task.objective_id.is_none());
    assert!(task.assignee.is_none());
    assert!(task.blocked_by.is_empty());
    assert!(task.due_at.is_none());
    assert!(task.duration_estimate.is_none());
    assert!(task.correlation_groups.is_empty());
    assert!(task.preconditions.is_none());
    assert!(task.effects.is_none());
    assert!(task.tags.is_empty());
    assert!(task.category_tag.is_none());
    assert!(task.occupies_capacity);
    assert!(task.static_window.is_none());
    task.validate().unwrap();
    let invalid = Task::new(source.id, source.title, TaskStatus::Moot, source.provenance);
    assert_eq!(invalid.validate(), Err(UbuError::MissingMootReasonCode));
}

#[test]
fn capacity_defaults_true_and_false_round_trips() {
    let task = base();
    assert!(task.occupies_capacity);
    let mut payload = serde_json::to_value(task).unwrap();
    assert!(payload.get("occupies_capacity").is_none());
    payload["occupies_capacity"] = json!(true);
    assert!(
        serde_json::from_value::<Task>(payload.clone())
            .unwrap()
            .occupies_capacity
    );
    payload["occupies_capacity"] = json!(false);
    let task: Task = serde_json::from_value(payload.clone()).unwrap();
    assert!(!task.occupies_capacity);
    assert_eq!(serde_json::to_value(task).unwrap(), payload);
}

#[test]
fn static_windows_require_strict_chronological_order_in_both_validation_paths() {
    for end in [
        "2026-09-21T09:59:59Z",
        "2026-09-21T10:00:00Z",
        "2026-09-21T06:00:00-04:00",
    ] {
        let invalid = window("2026-09-21T10:00:00Z", end);
        assert_eq!(invalid.validate(), Err(UbuError::InvalidTaskStaticWindow));
        let mut task = base();
        task.static_window = Some(invalid);
        assert_eq!(task.validate(), Err(UbuError::InvalidTaskStaticWindow));
        let error =
            serde_json::from_value::<Task>(serde_json::to_value(task).unwrap()).unwrap_err();
        assert!(error
            .to_string()
            .contains("static_window.end must be strictly after start"));
    }
    window("2026-09-21T10:00:00Z", "2026-09-21T06:00:00.001-04:00")
        .validate()
        .unwrap();
}

#[test]
fn category_selection_is_nonempty_exact_and_case_sensitive() {
    let mut task = base();
    task.tags = vec!["focus".into(), "writing".into()];
    for category in ["missing", "Focus", ""] {
        assert_eq!(
            validate_category_tag(category, &task.tags),
            Err(UbuError::InvalidTaskCategoryTag)
        );
        task.category_tag = Some(category.into());
        assert_eq!(task.validate(), Err(UbuError::InvalidTaskCategoryTag));
        assert!(serde_json::from_value::<Task>(serde_json::to_value(&task).unwrap()).is_err());
    }
    assert_eq!(
        validate_category_tag("", &[String::new()]),
        Err(UbuError::InvalidTaskCategoryTag)
    );
    task.category_tag = Some("focus".into());
    task.validate().unwrap();
    task.tags.clear();
    assert_eq!(task.validate(), Err(UbuError::InvalidTaskCategoryTag));
}

#[test]
fn static_duration_is_not_cross_validated_and_capacity_is_independent() {
    let mut task = base();
    task.static_window = Some(window("2026-09-21T10:00:00Z", "2026-09-21T11:00:00Z"));
    task.duration_estimate = Some(TaskDurationEstimate::Fixed { seconds: 1 });
    task.validate().unwrap();
    task.occupies_capacity = false;
    task.validate().unwrap();
    task.static_window = None;
    task.validate().unwrap();
}

#[test]
fn malformed_new_fields_are_rejected_without_changing_the_schema_contract() {
    let payload = serde_json::to_value(base()).unwrap();
    for field in [
        json!({"start":"2026-09-21T10:00:00Z"}),
        json!({"start":"invalid","end":"2026-09-21T11:00:00Z"}),
        json!({"start":"2026-09-21T10:00:00Z","end":"2026-09-21T11:00:00Z","extra":true}),
    ] {
        let mut invalid = payload.clone();
        invalid["static_window"] = field;
        assert!(serde_json::from_value::<Task>(invalid).is_err());
    }
    for value in [Value::Null, json!("false"), json!(0)] {
        let mut invalid = payload.clone();
        invalid["occupies_capacity"] = value;
        assert!(serde_json::from_value::<Task>(invalid).is_err());
    }
}
