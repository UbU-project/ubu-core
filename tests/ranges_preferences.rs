use serde_json::{json, Value};
use ubu_core::core::{AllowedTimeRange, Preference, PreferenceSubjects, Task};
use ubu_core::{ids::UbuId, ObjectType, UbuError};

fn pair() -> Value {
    serde_json::from_str(include_str!(
        "../fixtures/placeholders/valid/core/preference/task-pair.json"
    ))
    .unwrap()
}

#[test]
fn preference_subject_errors_are_named_at_deserialization_and_validation() {
    let mut mixed = pair();
    mixed["objective_b"] = json!(UbuId::new(ObjectType::Objective));
    let mut incomplete = pair();
    incomplete.as_object_mut().unwrap().remove("task_b");
    let mut wrong_kind = pair();
    wrong_kind["objective_a"] = wrong_kind["task_a"].take();
    wrong_kind["objective_b"] = json!(UbuId::new(ObjectType::Objective));
    wrong_kind.as_object_mut().unwrap().remove("task_a");
    wrong_kind.as_object_mut().unwrap().remove("task_b");
    let mut null_extra = pair();
    null_extra["objective_a"] = Value::Null;
    for value in [mixed, incomplete, wrong_kind, null_extra] {
        let error = serde_json::from_value::<Preference>(value).unwrap_err();
        assert!(
            error
                .to_string()
                .contains(&UbuError::InvalidPreferenceSubjects.to_string()),
            "{error}"
        );
    }
    let mut preference: Preference = serde_json::from_value(pair()).unwrap();
    preference.subjects = PreferenceSubjects::Objectives {
        a: UbuId::new(ObjectType::Task),
        b: UbuId::new(ObjectType::Objective),
    };
    assert_eq!(
        preference.validate(),
        Err(UbuError::InvalidPreferenceSubjects)
    );
}

#[test]
fn preference_self_relation_is_rejected() {
    let mut value = pair();
    value["task_b"] = value["task_a"].clone();
    let error = serde_json::from_value::<Preference>(value).unwrap_err();
    assert!(error
        .to_string()
        .contains(&UbuError::PreferenceSelfRelation.to_string()));
    let mut preference: Preference = serde_json::from_value(pair()).unwrap();
    let id = UbuId::new(ObjectType::Task);
    preference.subjects = PreferenceSubjects::Tasks {
        a: id.clone(),
        b: id,
    };
    assert_eq!(preference.validate(), Err(UbuError::PreferenceSelfRelation));
}

#[test]
fn range_order_and_scheduling_exclusivity_are_named() {
    let base: Value = serde_json::from_str(include_str!(
        "../fixtures/placeholders/valid/core/task/with-allowed-time-range.json"
    ))
    .unwrap();
    for latest in ["2026-09-21T12:00:00Z", "2026-09-21T11:00:00Z"] {
        let mut value = base.clone();
        value["allowed_time_range"]["latest_finish"] = json!(latest);
        let range: AllowedTimeRange =
            serde_json::from_value(value["allowed_time_range"].clone()).unwrap();
        assert_eq!(range.validate(), Err(UbuError::InvalidTaskAllowedTimeRange));
        let error = serde_json::from_value::<Task>(value).unwrap_err();
        assert!(error
            .to_string()
            .contains(&UbuError::InvalidTaskAllowedTimeRange.to_string()));
    }
    let mut both = base;
    both["static_window"] = json!({"start":"2026-09-21T12:00:00Z", "end":"2026-09-21T13:00:00Z"});
    let error = serde_json::from_value::<Task>(both).unwrap_err();
    assert!(error
        .to_string()
        .contains(&UbuError::TaskStaticWithAllowedTimeRange.to_string()));
}
