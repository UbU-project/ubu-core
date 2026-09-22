//! Phase 1b recurrence metadata; expansion and timezone resolution belong to the planner.
//! TODO(recurrence-interval): add interval semantics in a later contract.
//! TODO(recurrence-rdate): add explicit RDATE entries.
//! TODO(recurrence-overrides): add occurrence override entries.
use super::TaskDurationEstimate;
use crate::UbuId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveMode {
    #[default]
    OneTime,
    Evergreen,
}
impl ObjectiveMode {
    pub fn is_one_time(&self) -> bool {
        *self == Self::OneTime
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecurrenceSchedule {
    pub timezone: String,
    pub rule: RecurrenceRule,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled_until: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exdates: Vec<String>,
    pub schedule_version: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecurrenceRule {
    Daily,
    Weekly { weekdays: Vec<Weekday> },
    MonthlyDay { days: Vec<u8> },
    FirstWorkdayOfMonth,
    FirstWorkdayOfQuarter,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutinePlacement {
    Static,
    Planned,
}
impl std::fmt::Display for RoutinePlacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Static => "static",
            Self::Planned => "planned",
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalTimeRange {
    pub earliest: String,
    pub latest: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutineAfter {
    pub objective_id: UbuId,
    pub offset_seconds: i64,
}
fn yes() -> bool {
    true
}
fn is_true(value: &bool) -> bool {
    *value
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutineInstanceTemplate {
    pub title: String,
    pub duration_estimate: TaskDurationEstimate,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_tag: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reminder_minutes: Vec<i64>,
    pub nominal_start: String,
    pub placement: RoutinePlacement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_local_range: Option<LocalTimeRange>,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub occupies_capacity: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<RoutineAfter>,
    pub template_version: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskOccurrence {
    pub routine_objective_id: UbuId,
    pub local_date: String,
    pub key: String,
}

pub fn routine_occurrence_key(
    objective_id: &UbuId,
    schedule_version: u64,
    local_date: &str,
    nominal_start: &str,
    placement: RoutinePlacement,
    template_version: u64,
) -> String {
    format!("{objective_id}/s{schedule_version}/{local_date}T{nominal_start}/{placement}/t{template_version}")
}
