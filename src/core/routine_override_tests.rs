use crate::{core::*, UbuError, UbuId};
use serde_json::{json, Value};

fn schedule(overrides: Value) -> RecurrenceSchedule {
    serde_json::from_value(json!({"timezone":"UTC","rule":{"kind":"daily"},"schedule_version":1,"overrides":overrides})).unwrap()
}
fn entry() -> Value {
    json!({"local_date":"2026-09-27","start":"2026-09-27T15:00:00Z","end":"2026-09-27T15:30:00Z"})
}

#[test]
fn overrides_default_to_empty_and_lookup_round_trips() {
    let plain: RecurrenceSchedule = serde_json::from_value(
        json!({"timezone":"UTC","rule":{"kind":"daily"},"schedule_version":1}),
    )
    .unwrap();
    assert!(plain.overrides.is_empty());
    assert!(serde_json::to_value(plain)
        .unwrap()
        .get("overrides")
        .is_none());
    let schedule = schedule(json!([entry()]));
    schedule.validate().unwrap();
    assert_eq!(
        serde_json::to_value(schedule.override_for("2026-09-27").unwrap()).unwrap(),
        entry()
    );
    assert!(schedule.override_for("2026-09-28").is_none());
    let objective: Objective = serde_json::from_str(include_str!(
        "../../schemas-ref/fixtures/valid/core/objective/routine-override.json"
    ))
    .unwrap();
    objective.validate().unwrap();
}

#[test]
fn override_validation_rejects_duplicate_dates_invalid_dates_and_nonpositive_spans() {
    let mut other = entry();
    other["start"] = json!("2026-09-27T16:00:00Z");
    other["end"] = json!("2026-09-27T16:30:00Z");
    assert_eq!(
        schedule(json!([entry(), other])).validate(),
        Err(UbuError::DuplicateRoutineOverrideDate)
    );
    for date in ["bad", "2026-02-29", "2026-9-27"] {
        let mut invalid = entry();
        invalid["local_date"] = date.into();
        assert_eq!(
            schedule(json!([invalid])).validate(),
            Err(UbuError::InvalidRecurrenceDate)
        );
    }
    for end in [
        "2026-09-27T15:00:00Z",
        "2026-09-27T14:59:59Z",
        "2026-09-27T17:00:00+02:00",
    ] {
        let mut invalid = entry();
        invalid["end"] = end.into();
        assert_eq!(
            schedule(json!([invalid])).validate(),
            Err(UbuError::InvalidRoutineOverrideWindow)
        );
    }
}

#[test]
fn override_does_not_change_schedule_version_or_occurrence_key() {
    let mut schedule = schedule(json!([]));
    let id = UbuId::parse("obj_018f3c8e9b2a7c4d8f1e2a3b4c5d6e7f").unwrap();
    let before = routine_occurrence_key(
        &id,
        schedule.schedule_version,
        "2026-09-27",
        "07:00:00",
        RoutinePlacement::Planned,
        1,
    );
    schedule
        .overrides
        .push(serde_json::from_value(entry()).unwrap());
    assert_eq!(schedule.schedule_version, 1);
    assert_eq!(
        before,
        routine_occurrence_key(
            &id,
            schedule.schedule_version,
            "2026-09-27",
            "07:00:00",
            RoutinePlacement::Planned,
            1
        )
    );
}
