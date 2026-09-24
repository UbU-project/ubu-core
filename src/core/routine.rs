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
    /// Earliest this occurrence may start, measured from the predecessor's end.
    pub minimum_seconds: i64,
    /// Latest this occurrence may start, measured from the same point. Absent
    /// means the predecessor only orders this occurrence and does not keep it close.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_seconds: Option<i64>,
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

pub fn validate_timezone(value: &str) -> crate::Result<()> {
    static SHAPE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"^[A-Za-z_]+(/[A-Za-z0-9_+\-]+)+$").unwrap()
    });
    if value != "UTC" && !SHAPE.is_match(value) {
        return Err(crate::UbuError::InvalidRecurrenceTimezone);
    }
    Ok(())
}

pub fn validate_local_date(value: &str) -> crate::Result<()> {
    let format = time::format_description::parse_borrowed::<2>("[year]-[month]-[day]").unwrap();
    if value.len() != 10 || time::Date::parse(value, &format).is_err() {
        return Err(crate::UbuError::InvalidRecurrenceDate);
    }
    Ok(())
}

pub fn validate_local_time(value: &str) -> crate::Result<()> {
    let format = time::format_description::parse_borrowed::<2>("[hour]:[minute]:[second]").unwrap();
    if value.len() != 8 || time::Time::parse(value, &format).is_err() {
        return Err(crate::UbuError::InvalidRoutineLocalTime);
    }
    Ok(())
}

impl RecurrenceSchedule {
    pub fn validate(&self) -> crate::Result<()> {
        use crate::UbuError::*;
        use std::collections::BTreeSet;
        validate_timezone(&self.timezone)?;
        if self.schedule_version == 0 {
            return Err(ZeroScheduleVersion);
        }
        match &self.rule {
            RecurrenceRule::Weekly { weekdays } => {
                if weekdays.is_empty() {
                    return Err(EmptyRecurrenceWeekdays);
                }
                if weekdays.iter().collect::<BTreeSet<_>>().len() != weekdays.len() {
                    return Err(DuplicateRecurrenceWeekday);
                }
            }
            RecurrenceRule::MonthlyDay { days } => {
                if days.is_empty() {
                    return Err(EmptyRecurrenceDays);
                }
                if days.iter().any(|day| !(1..=31).contains(day)) {
                    return Err(InvalidRecurrenceDay);
                }
                if days.iter().collect::<BTreeSet<_>>().len() != days.len() {
                    return Err(DuplicateRecurrenceDay);
                }
            }
            _ => {}
        }
        for date in self
            .enabled_from
            .iter()
            .chain(self.enabled_until.iter())
            .chain(&self.exdates)
        {
            validate_local_date(date)?;
        }
        if let (Some(from), Some(until)) = (&self.enabled_from, &self.enabled_until) {
            if from > until {
                return Err(InvalidRecurrenceEnablementWindow);
            }
        }
        if self.exdates.iter().collect::<BTreeSet<_>>().len() != self.exdates.len() {
            return Err(DuplicateRecurrenceExdate);
        }
        Ok(())
    }
}
impl RoutineInstanceTemplate {
    pub fn validate(&self) -> crate::Result<()> {
        use crate::UbuError::*;
        if self.title.is_empty() {
            return Err(EmptyRoutineTitle);
        }
        self.duration_estimate.validate()?;
        validate_local_time(&self.nominal_start).map_err(|_| InvalidRoutineNominalStart)?;
        if self.template_version == 0 {
            return Err(ZeroTemplateVersion);
        }
        let mut tags = std::collections::BTreeSet::new();
        for tag in &self.tags {
            if tag.is_empty() {
                return Err(EmptyTaskTag);
            }
            if !tags.insert(tag) {
                return Err(DuplicateTaskTag);
            }
        }
        if let Some(category) = &self.category_tag {
            super::validate_category_tag(category, &self.tags)?;
        }
        if self.reminder_minutes.iter().any(|&n| n < 0) {
            return Err(NegativeRoutineReminder);
        }
        for after in &self.after {
            after
                .objective_id
                .require_object_type(crate::ObjectType::Objective)
                .map_err(|_| InvalidRoutineAfterObjective)?;
            if after.minimum_seconds < 0 {
                return Err(NegativeRoutineAfterOffset);
            }
            if after
                .maximum_seconds
                .is_some_and(|maximum| maximum < after.minimum_seconds)
            {
                return Err(InvertedRoutineAfterBounds);
            }
        }
        match (self.placement, &self.allowed_local_range) {
            (RoutinePlacement::Planned, None) => return Err(PlannedRoutineMissingRange),
            (RoutinePlacement::Static, Some(_)) => return Err(StaticRoutineWithRange),
            (_, Some(range)) => {
                validate_local_time(&range.earliest)?;
                validate_local_time(&range.latest)?;
                if range.earliest >= range.latest {
                    return Err(InvalidRoutineLocalRange);
                }
            }
            _ => {}
        }
        Ok(())
    }
}
impl TaskOccurrence {
    pub fn validate(&self) -> crate::Result<()> {
        self.routine_objective_id
            .require_object_type(crate::ObjectType::Objective)
            .map_err(|_| crate::UbuError::InvalidOccurrenceObjective)?;
        validate_local_date(&self.local_date)
            .map_err(|_| crate::UbuError::InvalidOccurrenceDate)?;
        if self.key.is_empty() {
            return Err(crate::UbuError::EmptyOccurrenceKey);
        }
        Ok(())
    }
}

/// Shared cross-field rules for full Objective validation and piecemeal admission.
pub fn validate_objective_routine_fields(
    id: &UbuId,
    mode: ObjectiveMode,
    recurrence: Option<&RecurrenceSchedule>,
    template: Option<&RoutineInstanceTemplate>,
    has_priority: bool,
) -> crate::Result<()> {
    use crate::UbuError::*;
    if recurrence.is_some() && mode != ObjectiveMode::Evergreen {
        return Err(RecurrenceRequiresEvergreen);
    }
    if template.is_some() && recurrence.is_none() {
        return Err(RoutineTemplateRequiresRecurrence);
    }
    if recurrence.is_some() && has_priority {
        return Err(RoutineObjectiveWithPriority);
    }
    if template.is_some_and(|t| t.after.iter().any(|after| after.objective_id == *id)) {
        return Err(RoutineAfterSelfReference);
    }
    Ok(())
}
