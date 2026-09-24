use serde_json::{json, Value};
use ubu_core::{core::*, UbuError, UbuId};
fn objective() -> Objective {
    serde_json::from_str(include_str!(
        "../fixtures/placeholders/valid/core/objective/routine-planned.json"
    ))
    .unwrap()
}
#[test]
fn after_bounds_reject_inversion_and_preserve_unbounded_edges() {
    let base = objective().routine_instance_template.unwrap();
    for maximum in [-1, 3599] {
        let mut template = base.clone();
        template.after[0].maximum_seconds = Some(maximum);
        assert_eq!(
            template.validate(),
            Err(UbuError::InvertedRoutineAfterBounds)
        );
    }
    for maximum in [None, Some(3600), Some(7200)] {
        let mut template = base.clone();
        template.after[0].maximum_seconds = maximum;
        template.validate().unwrap();
        let encoded = serde_json::to_value(&template.after[0]).unwrap();
        assert_eq!(encoded.get("maximum_seconds").is_some(), maximum.is_some());
        assert!(encoded.get("offset_seconds").is_none());
    }
}

#[test]
fn every_named_schedule_rule_is_validated() {
    let schedule = objective().recurrence.unwrap();
    for (field, value, expected) in [
        (
            "timezone",
            json!("Not a zone"),
            UbuError::InvalidRecurrenceTimezone,
        ),
        ("schedule_version", json!(0), UbuError::ZeroScheduleVersion),
        (
            "rule",
            json!({"kind":"weekly","weekdays":[]}),
            UbuError::EmptyRecurrenceWeekdays,
        ),
        (
            "rule",
            json!({"kind":"weekly","weekdays":["mon","mon"]}),
            UbuError::DuplicateRecurrenceWeekday,
        ),
        (
            "rule",
            json!({"kind":"monthly_day","days":[]}),
            UbuError::EmptyRecurrenceDays,
        ),
        (
            "rule",
            json!({"kind":"monthly_day","days":[0]}),
            UbuError::InvalidRecurrenceDay,
        ),
        (
            "rule",
            json!({"kind":"monthly_day","days":[32]}),
            UbuError::InvalidRecurrenceDay,
        ),
        (
            "rule",
            json!({"kind":"monthly_day","days":[1,1]}),
            UbuError::DuplicateRecurrenceDay,
        ),
        (
            "enabled_from",
            json!("2026-02-30"),
            UbuError::InvalidRecurrenceDate,
        ),
        (
            "enabled_from",
            json!("2028-01-01"),
            UbuError::InvalidRecurrenceEnablementWindow,
        ),
        (
            "exdates",
            json!(["2026-12-25", "2026-12-25"]),
            UbuError::DuplicateRecurrenceExdate,
        ),
    ] {
        let mut value_json = serde_json::to_value(&schedule).unwrap();
        value_json[field] = value;
        let schedule: RecurrenceSchedule = serde_json::from_value(value_json).unwrap();
        assert_eq!(schedule.validate(), Err(expected), "{field}");
    }
    for zone in [
        "UTC",
        "America/New_York",
        "Etc/GMT+5",
        "America/Argentina/Buenos_Aires",
    ] {
        validate_timezone(zone).unwrap();
    }
    for zone in ["", "local", "/New_York", "A/", "A//B", "A/B B"] {
        assert!(validate_timezone(zone).is_err());
    }
    for date in ["2026-2-01", "2026-02-29", "bad"] {
        assert!(validate_local_date(date).is_err());
    }
}
#[test]
fn every_named_template_rule_is_validated() {
    let template = objective().routine_instance_template.unwrap();
    let task_id = "task_018f3c8e9b2a7c4d8f1e2a3b4c5d6e80";
    for (field, value, expected) in [
        ("title", json!(""), UbuError::EmptyRoutineTitle),
        (
            "nominal_start",
            json!("24:00:00"),
            UbuError::InvalidRoutineNominalStart,
        ),
        ("template_version", json!(0), UbuError::ZeroTemplateVersion),
        ("tags", json!([""]), UbuError::EmptyTaskTag),
        (
            "tags",
            json!(["relationship", "relationship"]),
            UbuError::DuplicateTaskTag,
        ),
        (
            "category_tag",
            json!("missing"),
            UbuError::InvalidTaskCategoryTag,
        ),
        (
            "reminder_minutes",
            json!([-1]),
            UbuError::NegativeRoutineReminder,
        ),
        (
            "after",
            json!([{"objective_id":task_id,"minimum_seconds":0}]),
            UbuError::InvalidRoutineAfterObjective,
        ),
        (
            "after",
            json!([{"objective_id":objective().id,"minimum_seconds":-1}]),
            UbuError::NegativeRoutineAfterOffset,
        ),
        (
            "placement",
            json!("static"),
            UbuError::StaticRoutineWithRange,
        ),
        (
            "allowed_local_range",
            Value::Null,
            UbuError::PlannedRoutineMissingRange,
        ),
        (
            "allowed_local_range",
            json!({"earliest":"23:00:00","latest":"01:00:00"}),
            UbuError::InvalidRoutineLocalRange,
        ),
        (
            "allowed_local_range",
            json!({"earliest":"12:00:00","latest":"12:00:00"}),
            UbuError::InvalidRoutineLocalRange,
        ),
        (
            "allowed_local_range",
            json!({"earliest":"12:00","latest":"13:00:00"}),
            UbuError::InvalidRoutineLocalTime,
        ),
    ] {
        let mut v = serde_json::to_value(&template).unwrap();
        v[field] = value;
        let t: RoutineInstanceTemplate = serde_json::from_value(v).unwrap();
        assert_eq!(t.validate(), Err(expected));
    }
    let mut t = template;
    t.duration_estimate = TaskDurationEstimate::Fixed { seconds: 0 };
    assert!(t.validate().is_err());
}
#[test]
fn cross_fields_and_occurrence_rules_have_distinct_errors() {
    let base = objective();
    let mut o = base.clone();
    o.mode = ObjectiveMode::OneTime;
    assert_eq!(o.validate(), Err(UbuError::RecurrenceRequiresEvergreen));
    let mut o = base.clone();
    o.recurrence = None;
    assert_eq!(
        o.validate(),
        Err(UbuError::RoutineTemplateRequiresRecurrence)
    );
    let mut o = base.clone();
    o.priority = Some(1);
    assert_eq!(o.validate(), Err(UbuError::RoutineObjectiveWithPriority));
    let mut o = base.clone();
    o.routine_instance_template.as_mut().unwrap().after[0].objective_id = o.id.clone();
    assert_eq!(o.validate(), Err(UbuError::RoutineAfterSelfReference));
    let mut occurrence = TaskOccurrence {
        routine_objective_id: base.id.clone(),
        local_date: "2026-09-22".into(),
        key: "key".into(),
    };
    occurrence.validate().unwrap();
    occurrence.key.clear();
    assert_eq!(occurrence.validate(), Err(UbuError::EmptyOccurrenceKey));
    occurrence.key = "key".into();
    occurrence.local_date = "2026-02-30".into();
    assert_eq!(occurrence.validate(), Err(UbuError::InvalidOccurrenceDate));
    occurrence.local_date = "2026-09-22".into();
    occurrence.routine_objective_id = UbuId::new(ubu_core::ObjectType::Task);
    assert_eq!(
        occurrence.validate(),
        Err(UbuError::InvalidOccurrenceObjective)
    );
    assert_eq!(
        routine_occurrence_key(
            &base.id,
            1,
            "2026-09-22",
            "12:00:00",
            RoutinePlacement::Planned,
            2
        ),
        format!("{}/s1/2026-09-22T12:00:00/planned/t2", base.id)
    );
}
#[test]
fn defaults_are_omitted_and_objective_wire_accepts_compartment() {
    let mut v = serde_json::to_value(objective()).unwrap();
    v["compartment_id"] = json!("not modeled");
    serde_json::from_value::<Objective>(v).unwrap();
    let base = objective();
    let o = Objective::new(base.id, base.title, base.status, base.provenance);
    let v = serde_json::to_value(o).unwrap();
    assert!(v.get("mode").is_none());
    let static_objective: Objective = serde_json::from_str(include_str!(
        "../fixtures/placeholders/valid/core/objective/routine-static.json"
    ))
    .unwrap();
    let v = serde_json::to_value(static_objective).unwrap();
    assert!(v["routine_instance_template"]
        .get("occupies_capacity")
        .is_none());
}
