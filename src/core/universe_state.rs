use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryFrom;
use std::fmt;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Number, Value};
use thiserror::Error;

use crate::id_registry::ObjectType;
use crate::time::UbuTimestamp;
use crate::UbuId;

pub type UniverseFacts = BTreeMap<String, Value>;
pub type UniverseNumericValues = BTreeMap<String, f64>;
pub type UniverseSetMemberships = BTreeMap<String, BTreeSet<JsonScalar>>;
pub type UniverseEventMarkers = BTreeMap<String, Vec<Map<String, Value>>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniverseState {
    pub id: UbuId,
    pub captured_at: UbuTimestamp,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub facts: UniverseFacts,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub numeric_values: UniverseNumericValues,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub set_memberships: UniverseSetMemberships,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub event_markers: UniverseEventMarkers,
    pub source_summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence_summary: Option<String>,
}

impl UniverseState {
    pub fn new(captured_at: UbuTimestamp, source_summary: impl Into<String>) -> Self {
        Self {
            id: UbuId::new(ObjectType::UniverseState),
            captured_at,
            facts: BTreeMap::new(),
            numeric_values: BTreeMap::new(),
            set_memberships: BTreeMap::new(),
            event_markers: BTreeMap::new(),
            source_summary: source_summary.into(),
            confidence_summary: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum JsonScalar {
    Null,
    Bool(bool),
    Number(String),
    String(String),
}

impl TryFrom<&Value> for JsonScalar {
    type Error = JsonScalarError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Null => Ok(Self::Null),
            Value::Bool(value) => Ok(Self::Bool(*value)),
            Value::Number(value) => Ok(Self::Number(value.to_string())),
            Value::String(value) => Ok(Self::String(value.clone())),
            Value::Array(_) | Value::Object(_) => Err(JsonScalarError),
        }
    }
}

impl From<JsonScalar> for Value {
    fn from(value: JsonScalar) -> Self {
        match value {
            JsonScalar::Null => Value::Null,
            JsonScalar::Bool(value) => Value::Bool(value),
            JsonScalar::Number(value) => parse_json_number(&value)
                .map(Value::Number)
                .unwrap_or(Value::String(value)),
            JsonScalar::String(value) => Value::String(value),
        }
    }
}

impl Serialize for JsonScalar {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Null => serializer.serialize_none(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Number(value) => {
                let number = parse_json_number(value).map_err(serde::ser::Error::custom)?;
                number.serialize(serializer)
            }
            Self::String(value) => serializer.serialize_str(value),
        }
    }
}

impl<'de> Deserialize<'de> for JsonScalar {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Self::try_from(&value).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonScalarError;

impl fmt::Display for JsonScalarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected a JSON scalar")
    }
}

impl std::error::Error for JsonScalarError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniverseMutation {
    pub operation: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UniversePrecondition {
    AllOf { all_of: Vec<UniversePrecondition> },
    AnyOf { any_of: Vec<UniversePrecondition> },
    Leaf(UniversePreconditionLeaf),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniversePreconditionLeaf {
    pub target: String,
    pub predicate: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<Value>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum UniverseMutationError {
    #[error("mutation {index}: {message}")]
    InvalidItem { index: usize, message: String },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum UniversePreconditionError {
    #[error("malformed precondition: {0}")]
    Malformed(String),
}

pub fn apply_universe_mutations(
    state: &UniverseState,
    mutations: &[UniverseMutation],
) -> Result<UniverseState, UniverseMutationError> {
    let parsed = mutations
        .iter()
        .enumerate()
        .map(|(index, mutation)| validate_mutation(index, mutation))
        .collect::<Result<Vec<_>, _>>()?;

    let mut next = state.clone();
    for mutation in parsed {
        apply_valid_mutation(&mut next, mutation);
    }
    Ok(next)
}

pub fn evaluate_universe_precondition(
    state: &UniverseState,
    precondition: &UniversePrecondition,
) -> Result<bool, UniversePreconditionError> {
    match precondition {
        UniversePrecondition::AllOf { all_of } => {
            for child in all_of {
                if !evaluate_universe_precondition(state, child)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        UniversePrecondition::AnyOf { any_of } => {
            for child in any_of {
                if evaluate_universe_precondition(state, child)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        UniversePrecondition::Leaf(leaf) => evaluate_leaf_precondition(state, leaf),
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ValidMutation {
    SetFact {
        key: String,
        value: Value,
    },
    ClearFact {
        key: String,
    },
    IncrementNumeric {
        key: String,
        delta: f64,
    },
    DecrementNumeric {
        key: String,
        delta: f64,
    },
    AddMembership {
        key: String,
        member: JsonScalar,
    },
    RemoveMembership {
        key: String,
        member: JsonScalar,
    },
    AppendEventMarker {
        key: String,
        marker: Map<String, Value>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetCollection {
    Facts,
    NumericValues,
    SetMemberships,
    EventMarkers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedTarget {
    collection: TargetCollection,
    key: String,
}

fn validate_mutation(
    index: usize,
    mutation: &UniverseMutation,
) -> Result<ValidMutation, UniverseMutationError> {
    let target = parse_target(&mutation.target)
        .map_err(|message| UniverseMutationError::InvalidItem { index, message })?;

    match mutation.operation.as_str() {
        "set_fact" => {
            require_collection(index, target.collection, TargetCollection::Facts)?;
            let value = require_payload(index, mutation)?.clone();
            Ok(ValidMutation::SetFact {
                key: target.key,
                value,
            })
        }
        "clear_fact" => {
            require_collection(index, target.collection, TargetCollection::Facts)?;
            if mutation.payload.is_some() {
                return Err(invalid_mutation(
                    index,
                    "clear_fact does not accept a payload",
                ));
            }
            Ok(ValidMutation::ClearFact { key: target.key })
        }
        "increment_numeric" => {
            require_collection(index, target.collection, TargetCollection::NumericValues)?;
            Ok(ValidMutation::IncrementNumeric {
                key: target.key,
                delta: require_numeric_payload(index, mutation)?,
            })
        }
        "decrement_numeric" => {
            require_collection(index, target.collection, TargetCollection::NumericValues)?;
            Ok(ValidMutation::DecrementNumeric {
                key: target.key,
                delta: require_numeric_payload(index, mutation)?,
            })
        }
        "add_membership" => {
            require_collection(index, target.collection, TargetCollection::SetMemberships)?;
            Ok(ValidMutation::AddMembership {
                key: target.key,
                member: require_scalar_payload(index, mutation)?,
            })
        }
        "remove_membership" => {
            require_collection(index, target.collection, TargetCollection::SetMemberships)?;
            Ok(ValidMutation::RemoveMembership {
                key: target.key,
                member: require_scalar_payload(index, mutation)?,
            })
        }
        "append_event_marker" => {
            require_collection(index, target.collection, TargetCollection::EventMarkers)?;
            let marker = require_payload(index, mutation)?
                .as_object()
                .cloned()
                .ok_or_else(|| {
                    invalid_mutation(index, "append_event_marker payload must be a JSON object")
                })?;
            Ok(ValidMutation::AppendEventMarker {
                key: target.key,
                marker,
            })
        }
        operation => Err(invalid_mutation(
            index,
            format!("unknown operation `{operation}`"),
        )),
    }
}

fn apply_valid_mutation(state: &mut UniverseState, mutation: ValidMutation) {
    match mutation {
        ValidMutation::SetFact { key, value } => {
            state.facts.insert(key, value);
        }
        ValidMutation::ClearFact { key } => {
            state.facts.remove(&key);
        }
        ValidMutation::IncrementNumeric { key, delta } => {
            *state.numeric_values.entry(key).or_insert(0.0) += delta;
        }
        ValidMutation::DecrementNumeric { key, delta } => {
            *state.numeric_values.entry(key).or_insert(0.0) -= delta;
        }
        ValidMutation::AddMembership { key, member } => {
            state.set_memberships.entry(key).or_default().insert(member);
        }
        ValidMutation::RemoveMembership { key, member } => {
            if let Some(members) = state.set_memberships.get_mut(&key) {
                members.remove(&member);
                if members.is_empty() {
                    state.set_memberships.remove(&key);
                }
            }
        }
        ValidMutation::AppendEventMarker { key, marker } => {
            state.event_markers.entry(key).or_default().push(marker);
        }
    }
}

fn evaluate_leaf_precondition(
    state: &UniverseState,
    leaf: &UniversePreconditionLeaf,
) -> Result<bool, UniversePreconditionError> {
    let target = parse_target(&leaf.target).map_err(UniversePreconditionError::Malformed)?;

    match leaf.predicate.as_str() {
        "equals" => {
            let expected = leaf.expected.as_ref().ok_or_else(|| {
                UniversePreconditionError::Malformed("equals requires expected".to_owned())
            })?;
            Ok(target_equals(state, &target, expected))
        }
        "member_of" => {
            if target.collection != TargetCollection::SetMemberships {
                return Err(UniversePreconditionError::Malformed(
                    "member_of requires a set_memberships target".to_owned(),
                ));
            }
            let expected = leaf.expected.as_ref().ok_or_else(|| {
                UniversePreconditionError::Malformed("member_of requires expected".to_owned())
            })?;
            let expected = JsonScalar::try_from(expected).map_err(|_| {
                UniversePreconditionError::Malformed(
                    "member_of expected value must be a JSON scalar".to_owned(),
                )
            })?;
            Ok(state
                .set_memberships
                .get(&target.key)
                .is_some_and(|members| members.contains(&expected)))
        }
        "absent" => Ok(value_at_target(state, &target).is_none()),
        predicate => Err(UniversePreconditionError::Malformed(format!(
            "unknown predicate `{predicate}`"
        ))),
    }
}

fn value_at_target(state: &UniverseState, target: &ParsedTarget) -> Option<Value> {
    match target.collection {
        TargetCollection::Facts => state.facts.get(&target.key).cloned(),
        TargetCollection::NumericValues => state
            .numeric_values
            .get(&target.key)
            .and_then(|value| Number::from_f64(*value))
            .map(Value::Number),
        TargetCollection::SetMemberships => state
            .set_memberships
            .get(&target.key)
            .map(|members| Value::Array(members.iter().cloned().map(Value::from).collect())),
        TargetCollection::EventMarkers => state
            .event_markers
            .get(&target.key)
            .map(|markers| Value::Array(markers.iter().cloned().map(Value::Object).collect())),
    }
}

fn target_equals(state: &UniverseState, target: &ParsedTarget, expected: &Value) -> bool {
    if target.collection == TargetCollection::NumericValues {
        return state
            .numeric_values
            .get(&target.key)
            .zip(expected.as_f64())
            .is_some_and(|(actual, expected)| *actual == expected);
    }

    value_at_target(state, target)
        .as_ref()
        .is_some_and(|actual| actual == expected)
}

fn require_collection(
    index: usize,
    actual: TargetCollection,
    expected: TargetCollection,
) -> Result<(), UniverseMutationError> {
    if actual == expected {
        Ok(())
    } else {
        Err(invalid_mutation(
            index,
            format!(
                "operation target must be in the {} collection",
                expected.as_str()
            ),
        ))
    }
}

fn require_payload<'a>(
    index: usize,
    mutation: &'a UniverseMutation,
) -> Result<&'a Value, UniverseMutationError> {
    mutation
        .payload
        .as_ref()
        .ok_or_else(|| invalid_mutation(index, "operation requires a payload"))
}

fn require_numeric_payload(
    index: usize,
    mutation: &UniverseMutation,
) -> Result<f64, UniverseMutationError> {
    require_payload(index, mutation)?
        .as_f64()
        .ok_or_else(|| invalid_mutation(index, "payload must be a JSON number"))
}

fn require_scalar_payload(
    index: usize,
    mutation: &UniverseMutation,
) -> Result<JsonScalar, UniverseMutationError> {
    JsonScalar::try_from(require_payload(index, mutation)?)
        .map_err(|_| invalid_mutation(index, "payload must be a JSON scalar"))
}

fn invalid_mutation(index: usize, message: impl Into<String>) -> UniverseMutationError {
    UniverseMutationError::InvalidItem {
        index,
        message: message.into(),
    }
}

fn parse_target(target: &str) -> Result<ParsedTarget, String> {
    let (collection, key) = target
        .split_once('.')
        .ok_or_else(|| format!("malformed target `{target}`"))?;
    if key.is_empty() || key.split('.').any(str::is_empty) {
        return Err(format!("malformed target `{target}`"));
    }

    let collection = match collection {
        "facts" => TargetCollection::Facts,
        "numeric_values" => TargetCollection::NumericValues,
        "set_memberships" => TargetCollection::SetMemberships,
        "event_markers" => TargetCollection::EventMarkers,
        _ => return Err(format!("malformed target `{target}`")),
    };

    Ok(ParsedTarget {
        collection,
        key: key.to_owned(),
    })
}

impl TargetCollection {
    fn as_str(self) -> &'static str {
        match self {
            Self::Facts => "facts",
            Self::NumericValues => "numeric_values",
            Self::SetMemberships => "set_memberships",
            Self::EventMarkers => "event_markers",
        }
    }
}

fn parse_json_number(value: &str) -> Result<Number, String> {
    value
        .parse::<u64>()
        .map(Number::from)
        .or_else(|_| value.parse::<i64>().map(Number::from))
        .or_else(|_| {
            value
                .parse::<f64>()
                .ok()
                .and_then(Number::from_f64)
                .ok_or_else(|| format!("invalid JSON number `{value}`"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn state() -> UniverseState {
        UniverseState::new(
            UbuTimestamp::parse("2026-06-22T12:00:00Z").expect("valid timestamp"),
            "test fixture",
        )
    }

    fn mutation(operation: &str, target: &str, payload: Option<Value>) -> UniverseMutation {
        UniverseMutation {
            operation: operation.to_owned(),
            target: target.to_owned(),
            payload,
            note: None,
        }
    }

    fn leaf(target: &str, predicate: &str, expected: Option<Value>) -> UniversePrecondition {
        UniversePrecondition::Leaf(UniversePreconditionLeaf {
            target: target.to_owned(),
            predicate: predicate.to_owned(),
            expected,
        })
    }

    #[test]
    fn set_fact_sets_any_json_value() {
        let next = apply_universe_mutations(
            &state(),
            &[mutation(
                "set_fact",
                "facts.ticket.status",
                Some(json!({"state": "ready"})),
            )],
        )
        .expect("valid mutation");

        assert_eq!(
            next.facts.get("ticket.status"),
            Some(&json!({"state": "ready"}))
        );
    }

    #[test]
    fn clear_fact_removes_existing_fact() {
        let initial = apply_universe_mutations(
            &state(),
            &[mutation(
                "set_fact",
                "facts.ticket.status",
                Some(json!("ready")),
            )],
        )
        .expect("valid mutation");

        let next = apply_universe_mutations(
            &initial,
            &[mutation("clear_fact", "facts.ticket.status", None)],
        )
        .expect("valid mutation");

        assert!(!next.facts.contains_key("ticket.status"));
    }

    #[test]
    fn increment_numeric_initializes_missing_key_from_zero() {
        let next = apply_universe_mutations(
            &state(),
            &[mutation(
                "increment_numeric",
                "numeric_values.energy",
                Some(json!(2.5)),
            )],
        )
        .expect("valid mutation");

        assert_eq!(next.numeric_values.get("energy"), Some(&2.5));
    }

    #[test]
    fn decrement_numeric_initializes_missing_key_from_zero() {
        let next = apply_universe_mutations(
            &state(),
            &[mutation(
                "decrement_numeric",
                "numeric_values.energy",
                Some(json!(2.5)),
            )],
        )
        .expect("valid mutation");

        assert_eq!(next.numeric_values.get("energy"), Some(&-2.5));
    }

    #[test]
    fn add_membership_adds_scalar_member() {
        let next = apply_universe_mutations(
            &state(),
            &[mutation(
                "add_membership",
                "set_memberships.tags",
                Some(json!("focused")),
            )],
        )
        .expect("valid mutation");

        assert!(next
            .set_memberships
            .get("tags")
            .expect("set exists")
            .contains(&JsonScalar::String("focused".to_owned())));
    }

    #[test]
    fn remove_membership_removes_scalar_member() {
        let initial = apply_universe_mutations(
            &state(),
            &[mutation(
                "add_membership",
                "set_memberships.tags",
                Some(json!("focused")),
            )],
        )
        .expect("valid mutation");

        let next = apply_universe_mutations(
            &initial,
            &[mutation(
                "remove_membership",
                "set_memberships.tags",
                Some(json!("focused")),
            )],
        )
        .expect("valid mutation");

        assert!(!next.set_memberships.contains_key("tags"));
    }

    #[test]
    fn append_event_marker_appends_to_empty_list() {
        let next = apply_universe_mutations(
            &state(),
            &[mutation(
                "append_event_marker",
                "event_markers.task.completed",
                Some(json!({"task_id": "task_1", "source": "test"})),
            )],
        )
        .expect("valid mutation");

        assert_eq!(
            next.event_markers
                .get("task.completed")
                .and_then(|markers| markers.first())
                .cloned()
                .map(Value::Object),
            Some(json!({"task_id": "task_1", "source": "test"}))
        );
    }

    #[test]
    fn invalid_item_rejects_whole_list_without_partial_apply() {
        let initial = state();
        let result = apply_universe_mutations(
            &initial,
            &[
                mutation("set_fact", "facts.ticket.status", Some(json!("ready"))),
                mutation("increment_numeric", "facts.energy", Some(json!(1))),
            ],
        );

        assert!(result.is_err());
        assert!(initial.facts.is_empty());
    }

    #[test]
    fn mutations_apply_in_list_order() {
        let next = apply_universe_mutations(
            &state(),
            &[
                mutation("increment_numeric", "numeric_values.energy", Some(json!(5))),
                mutation("decrement_numeric", "numeric_values.energy", Some(json!(2))),
                mutation("set_fact", "facts.ticket.status", Some(json!("ready"))),
                mutation("set_fact", "facts.ticket.status", Some(json!("done"))),
            ],
        )
        .expect("valid mutations");

        assert_eq!(next.numeric_values.get("energy"), Some(&3.0));
        assert_eq!(next.facts.get("ticket.status"), Some(&json!("done")));
    }

    #[test]
    fn equals_predicate_uses_json_compatible_equality() {
        let next = apply_universe_mutations(
            &state(),
            &[
                mutation("set_fact", "facts.ticket.status", Some(json!("ready"))),
                mutation("increment_numeric", "numeric_values.energy", Some(json!(2))),
            ],
        )
        .expect("valid mutations");

        assert_eq!(
            evaluate_universe_precondition(
                &next,
                &leaf("facts.ticket.status", "equals", Some(json!("ready")))
            ),
            Ok(true)
        );
        assert_eq!(
            evaluate_universe_precondition(
                &next,
                &leaf("numeric_values.energy", "equals", Some(json!(2.0)))
            ),
            Ok(true)
        );
    }

    #[test]
    fn member_of_predicate_checks_set_membership() {
        let next = apply_universe_mutations(
            &state(),
            &[mutation(
                "add_membership",
                "set_memberships.tags",
                Some(json!("focused")),
            )],
        )
        .expect("valid mutation");

        assert_eq!(
            evaluate_universe_precondition(
                &next,
                &leaf("set_memberships.tags", "member_of", Some(json!("focused")))
            ),
            Ok(true)
        );
        assert_eq!(
            evaluate_universe_precondition(
                &next,
                &leaf("set_memberships.tags", "member_of", Some(json!("blocked")))
            ),
            Ok(false)
        );
    }

    #[test]
    fn absent_predicate_is_true_for_missing_or_cleared_targets() {
        let initial = apply_universe_mutations(
            &state(),
            &[mutation(
                "set_fact",
                "facts.ticket.status",
                Some(json!("ready")),
            )],
        )
        .expect("valid mutation");
        let next = apply_universe_mutations(
            &initial,
            &[mutation("clear_fact", "facts.ticket.status", None)],
        )
        .expect("valid mutation");

        assert_eq!(
            evaluate_universe_precondition(&next, &leaf("facts.ticket.status", "absent", None)),
            Ok(true)
        );
        assert_eq!(
            evaluate_universe_precondition(&next, &leaf("facts.ticket.owner", "absent", None)),
            Ok(true)
        );
    }

    #[test]
    fn all_of_and_any_of_compose_recursively() {
        let next = apply_universe_mutations(
            &state(),
            &[
                mutation("set_fact", "facts.ticket.status", Some(json!("ready"))),
                mutation(
                    "add_membership",
                    "set_memberships.tags",
                    Some(json!("focused")),
                ),
            ],
        )
        .expect("valid mutations");

        let precondition = UniversePrecondition::AllOf {
            all_of: vec![
                leaf("facts.ticket.status", "equals", Some(json!("ready"))),
                UniversePrecondition::AnyOf {
                    any_of: vec![
                        leaf("set_memberships.tags", "member_of", Some(json!("blocked"))),
                        leaf("set_memberships.tags", "member_of", Some(json!("focused"))),
                    ],
                },
            ],
        };

        assert_eq!(
            evaluate_universe_precondition(&next, &precondition),
            Ok(true)
        );
    }

    #[test]
    fn unknown_or_partially_modeled_target_is_absent() {
        let next = apply_universe_mutations(
            &state(),
            &[mutation("set_fact", "facts.ticket", Some(json!("modeled")))],
        )
        .expect("valid mutation");

        assert_eq!(
            evaluate_universe_precondition(&next, &leaf("facts.ticket.status", "absent", None)),
            Ok(true)
        );
        assert_eq!(
            evaluate_universe_precondition(
                &next,
                &leaf("facts.ticket.status", "equals", Some(json!("ready")))
            ),
            Ok(false)
        );
    }

    #[test]
    fn numeric_targets_support_only_equality_and_absence() {
        let next = apply_universe_mutations(
            &state(),
            &[mutation(
                "increment_numeric",
                "numeric_values.energy",
                Some(json!(1)),
            )],
        )
        .expect("valid mutation");

        assert_eq!(
            evaluate_universe_precondition(
                &next,
                &leaf("numeric_values.energy", "equals", Some(json!(1.0)))
            ),
            Ok(true)
        );
        assert_eq!(
            evaluate_universe_precondition(&next, &leaf("numeric_values.missing", "absent", None)),
            Ok(true)
        );
        assert!(evaluate_universe_precondition(
            &next,
            &leaf("numeric_values.energy", "member_of", Some(json!(1.0)))
        )
        .is_err());
    }

    #[test]
    fn malformed_precondition_is_error_not_false() {
        assert!(evaluate_universe_precondition(
            &state(),
            &leaf("facts..status", "equals", Some(json!("ready")))
        )
        .is_err());
        assert!(evaluate_universe_precondition(
            &state(),
            &leaf("facts.status", "greater_than", Some(json!(1)))
        )
        .is_err());
        assert!(
            evaluate_universe_precondition(&state(), &leaf("facts.status", "equals", None))
                .is_err()
        );
    }

    #[test]
    fn mutation_validation_rejects_wrong_payload_types() {
        assert!(apply_universe_mutations(
            &state(),
            &[mutation(
                "add_membership",
                "set_memberships.tags",
                Some(json!(["not", "scalar"]))
            )]
        )
        .is_err());
        assert!(apply_universe_mutations(
            &state(),
            &[mutation(
                "append_event_marker",
                "event_markers.audit",
                Some(json!("not object"))
            )]
        )
        .is_err());
        assert!(
            apply_universe_mutations(&state(), &[mutation("set_fact", "facts.status", None)])
                .is_err()
        );
    }
}
