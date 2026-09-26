use thiserror::Error;

pub type Result<T> = std::result::Result<T, UbuError>;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum DurationEstimateViolation {
    #[error("duration_estimate.fixed.seconds must be greater than zero")]
    ZeroSeconds,
    #[error("duration_estimate must satisfy min_seconds < mode_seconds < p95_seconds")]
    NotStrictlyIncreasing,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum CorrelationGroupViolation {
    #[error("correlation group strength must be between zero and one")]
    StrengthOutOfRange,
    #[error("correlation group names must be unique")]
    DuplicateName,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UbuError {
    #[error("recurrence timezone must be UTC or Area/Location")]
    InvalidRecurrenceTimezone,
    #[error("recurrence date must be a valid YYYY-MM-DD date")]
    InvalidRecurrenceDate,
    #[error("schedule_version must be positive")]
    ZeroScheduleVersion,
    #[error("weekly recurrence requires nonempty weekdays")]
    EmptyRecurrenceWeekdays,
    #[error("recurrence weekdays must be unique")]
    DuplicateRecurrenceWeekday,
    #[error("monthly_day recurrence requires nonempty days")]
    EmptyRecurrenceDays,
    #[error("recurrence days must be 1 through 31")]
    InvalidRecurrenceDay,
    #[error("recurrence days must be unique")]
    DuplicateRecurrenceDay,
    #[error("enabled_from must not be after enabled_until")]
    InvalidRecurrenceEnablementWindow,
    #[error("exdates must be unique")]
    DuplicateRecurrenceExdate,
    #[error("routine overrides must have unique local_date values")]
    DuplicateRoutineOverrideDate,
    #[error("routine override end must be after start")]
    InvalidRoutineOverrideWindow,
    #[error("routine template title must not be empty")]
    EmptyRoutineTitle,
    #[error("local time must be valid HH:MM:SS")]
    InvalidRoutineLocalTime,
    #[error("nominal_start must be valid HH:MM:SS")]
    InvalidRoutineNominalStart,
    #[error("template_version must be positive")]
    ZeroTemplateVersion,
    #[error("reminder_minutes must be nonnegative")]
    NegativeRoutineReminder,
    #[error("after objective_id must be an Objective id")]
    InvalidRoutineAfterObjective,
    #[error("after minimum_seconds must be nonnegative")]
    NegativeRoutineAfterOffset,
    #[error("after maximum_seconds must be at least minimum_seconds")]
    InvertedRoutineAfterBounds,
    #[error("planned routine requires allowed_local_range")]
    PlannedRoutineMissingRange,
    #[error("static routine forbids allowed_local_range")]
    StaticRoutineWithRange,
    #[error("allowed_local_range earliest must precede latest on the same day")]
    InvalidRoutineLocalRange,
    #[error("routine_objective_id must be an Objective id")]
    InvalidOccurrenceObjective,
    #[error("occurrence local_date must be valid YYYY-MM-DD")]
    InvalidOccurrenceDate,
    #[error("occurrence key must not be empty")]
    EmptyOccurrenceKey,
    #[error("recurrence requires evergreen mode")]
    RecurrenceRequiresEvergreen,
    #[error("routine_instance_template requires recurrence")]
    RoutineTemplateRequiresRecurrence,
    #[error("routine Objective must not carry priority")]
    RoutineObjectiveWithPriority,
    #[error("routine after must not name its own Objective")]
    RoutineAfterSelfReference,

    #[error("Preference subjects must be exactly one complete pair of Task IDs or Objective IDs")]
    InvalidPreferenceSubjects,
    #[error("Preference cannot relate a subject to itself")]
    PreferenceSelfRelation,
    #[error("invalid local advisory field `{field}`")]
    InvalidLocalAdvisory { field: &'static str },
    #[error("advisory capability is not granted for candidate kind `{kind}`")]
    UngrantedAdvisoryCapability { kind: String },
    #[error("local advisory result exceeds result_size_limit_bytes")]
    AdvisoryResultTooLarge,

    #[error("suppression requires a rejected candidate, got {state:?}")]
    CandidateNotRejected {
        state: crate::advisory_candidate::CandidateLifecycleState,
    },

    #[error("invalid suppression record field `{field}`")]
    InvalidSuppressionRecord { field: &'static str },

    #[error("invalid advisory candidate field `{field}`")]
    InvalidCandidateRecord { field: &'static str },

    #[error("invalid candidate transition from {current:?} to {next:?}")]
    InvalidCandidateTransition {
        current: crate::advisory_candidate::CandidateLifecycleState,
        next: crate::advisory_candidate::CandidateLifecycleState,
    },

    #[error("resurface trigger must be supplied exactly when entering Resurfaced: {current:?} -> {next:?}")]
    InvalidResurfaceTrigger {
        current: crate::advisory_candidate::CandidateLifecycleState,
        next: crate::advisory_candidate::CandidateLifecycleState,
    },

    #[error("invalid advisory candidate id `{value}`")]
    InvalidAdvisoryCandidateId { value: String },

    #[error("zone id must not be empty")]
    EmptyZoneId,

    #[error("device label must not be empty")]
    EmptyDeviceLabel,

    #[error("device capability must not be empty")]
    EmptyDeviceCapability,

    #[error("duplicate device id `{device_id}` in registration registry")]
    DuplicateDeviceId { device_id: String },

    #[error("origin device id must not be empty")]
    EmptyDeviceId,

    #[error("idempotency key must not be empty")]
    EmptyIdempotencyKey,

    #[error("invalid version reference `{value}`")]
    InvalidVersionRef { value: String },

    #[error("policy-dependent mutation requires non-empty observed_policy_versions")]
    MissingObservedPolicyVersions,

    #[error(
        "idempotency_key_conflict for device `{origin_device_id}` and key `{idempotency_key}`"
    )]
    IdempotencyKeyConflict {
        origin_device_id: String,
        idempotency_key: String,
    },

    #[error("invalid UbU id `{value}`")]
    InvalidId { value: String },

    #[error("unknown UbU id prefix `{prefix}`")]
    UnknownIdPrefix { prefix: String },

    #[error("id `{id}` has object type `{actual}`, expected `{expected}`")]
    WrongIdObjectType {
        id: String,
        expected: &'static str,
        actual: &'static str,
    },

    #[error("invalid RFC 3339 timestamp with required timezone offset `{value}`")]
    InvalidTimestamp { value: String },

    #[error("invalid compartment label `{value}`")]
    InvalidCompartmentLabel { value: String },

    #[error("task status `moot` requires moot_reason_code")]
    MissingMootReasonCode,

    #[error("task status `{status}` forbids moot_reason_code")]
    UnexpectedMootReasonCode { status: &'static str },

    #[error("Task assignee must be an Identity id")]
    InvalidTaskAssignee,
    #[error("Task blocked_by contains a duplicate Task id")]
    DuplicateTaskBlockedBy,
    #[error("Task blocked_by contains a non-Task id")]
    InvalidTaskBlockedBy,
    #[error("Task cannot be blocked by itself")]
    TaskSelfBlocked,
    #[error("core/task schema: {violation}")]
    InvalidTaskDurationEstimate {
        violation: DurationEstimateViolation,
    },
    #[error("Task tags must not contain empty strings")]
    EmptyTaskTag,
    #[error("Task tags must be unique")]
    DuplicateTaskTag,
    #[error("Task static_window.end must be strictly after start")]
    InvalidTaskStaticWindow,
    #[error("Task allowed_time_range.earliest_start must be strictly before latest_finish")]
    InvalidTaskAllowedTimeRange,
    #[error("Task cannot have both static_window and allowed_time_range")]
    TaskStaticWithAllowedTimeRange,
    #[error("Task category_tag must be non-empty and an exact member of tags")]
    InvalidTaskCategoryTag,
    #[error("core/task schema: {violation}")]
    InvalidTaskCorrelationGroup {
        violation: CorrelationGroupViolation,
    },
}
