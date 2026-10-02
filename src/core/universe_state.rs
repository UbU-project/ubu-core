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
/// Keyed by the full target, `<collection>.<key>`, in the spelling a mutation's
/// `target` and a precondition's `target` use.
pub type UniverseFactProvenance = BTreeMap<String, FactProvenance>;

/// How a value in a `UniverseState` was established.
///
/// The point of the distinction is evidence against assertion, so there is no
/// score beside it: a number would blur exactly what this separates.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceKind {
    /// A person said so. What a mutation that states no kind records.
    #[default]
    Asserted,
    /// An instrument or a reading.
    Measured,
    /// Computed from other facts.
    Derived,
    /// An advisor suggested it and it has not been confirmed.
    Proposed,
}

impl ProvenanceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Asserted => "asserted",
            Self::Measured => "measured",
            Self::Derived => "derived",
            Self::Proposed => "proposed",
        }
    }
}

/// How one value was established, and when. Deliberately nothing else: no
/// confidence number and no free text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactProvenance {
    pub kind: ProvenanceKind,
    /// When the value was written. Independent of the state's `captured_at`.
    pub recorded_at: UbuTimestamp,
}

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
    /// Per-target provenance, beside the four value maps and not wrapping them.
    /// It is not named `provenance`: a stored `UniverseState` payload carries the
    /// object envelope under that key, and the store rewrites it on each write.
    /// An entry never outlives its value: the applicator removes both together.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fact_provenance: UniverseFactProvenance,
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
            fact_provenance: BTreeMap::new(),
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

/// One change to a `UniverseState`.
///
/// There is no `note`. It was accepted and stored nowhere, and a field that
/// silently discards what it is given is worse than no field. A mutation that
/// carries one, or any other unknown key, does not deserialize.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniverseMutation {
    pub operation: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    /// How the value this mutation writes was established. Absent means
    /// `asserted`: a mutation with no stated evidence is someone's word.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance_kind: Option<ProvenanceKind>,
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

/// §5 instance mode. `user_mode` models intrinsic affect; `organization_mode`
/// and `worker_mode` do not and reject intrinsic-affect targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
// Variant names mirror the §5 `*_mode` instance modes; the shared suffix is intentional.
#[allow(clippy::enum_variant_names)]
pub enum InstanceMode {
    UserMode,
    OrganizationMode,
    WorkerMode,
}

impl InstanceMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserMode => "user_mode",
            Self::OrganizationMode => "organization_mode",
            Self::WorkerMode => "worker_mode",
        }
    }

    /// Whether this mode models intrinsic affect and therefore permits
    /// intrinsic-affect targets.
    pub fn models_intrinsic_affect(self) -> bool {
        matches!(self, Self::UserMode)
    }
}

impl fmt::Display for InstanceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Rejection raised when a target addresses intrinsic affect in a mode that
/// does not model it. Distinct from precondition evaluation and mutation
/// application errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ModeValidationError {
    #[error("instance mode `{mode}` does not model intrinsic affect; target `{target}` is not permitted")]
    IntrinsicAffectForbidden { mode: InstanceMode, target: String },
}

/// A dotted target is intrinsic-affect when the namespace segment immediately
/// after its collection is `affect` (e.g. `numeric_values.affect.energy`). The
/// collection is the first segment; the namespace begins at the second.
pub fn is_intrinsic_affect_target(target: &str) -> bool {
    target
        .split('.')
        .nth(1)
        .is_some_and(|namespace| namespace == "affect")
}

/// Reject intrinsic-affect leaf targets anywhere in a precondition tree when
/// the mode does not model intrinsic affect. `user_mode` always passes.
pub fn validate_precondition_for_mode(
    mode: InstanceMode,
    precondition: &UniversePrecondition,
) -> Result<(), ModeValidationError> {
    if mode.models_intrinsic_affect() {
        return Ok(());
    }
    match first_intrinsic_affect_precondition_target(precondition) {
        Some(target) => Err(ModeValidationError::IntrinsicAffectForbidden {
            mode,
            target: target.to_owned(),
        }),
        None => Ok(()),
    }
}

/// Reject intrinsic-affect mutation targets when the mode does not model
/// intrinsic affect. `user_mode` always passes.
pub fn validate_mutations_for_mode(
    mode: InstanceMode,
    mutations: &[UniverseMutation],
) -> Result<(), ModeValidationError> {
    if mode.models_intrinsic_affect() {
        return Ok(());
    }
    match mutations
        .iter()
        .find(|mutation| is_intrinsic_affect_target(&mutation.target))
    {
        Some(mutation) => Err(ModeValidationError::IntrinsicAffectForbidden {
            mode,
            target: mutation.target.clone(),
        }),
        None => Ok(()),
    }
}

fn first_intrinsic_affect_precondition_target(precondition: &UniversePrecondition) -> Option<&str> {
    match precondition {
        UniversePrecondition::AllOf { all_of } => all_of
            .iter()
            .find_map(first_intrinsic_affect_precondition_target),
        UniversePrecondition::AnyOf { any_of } => any_of
            .iter()
            .find_map(first_intrinsic_affect_precondition_target),
        UniversePrecondition::Leaf(leaf) => {
            is_intrinsic_affect_target(&leaf.target).then_some(leaf.target.as_str())
        }
    }
}

/// Apply `mutations` in order, or none of them: the whole list is validated
/// before anything changes.
///
/// `recorded_at` is the write time each written target's provenance carries. It
/// is the caller's clock and not read here, so the same inputs give the same
/// state. It does not move the state's `captured_at`.
pub fn apply_universe_mutations(
    state: &UniverseState,
    mutations: &[UniverseMutation],
    recorded_at: UbuTimestamp,
) -> Result<UniverseState, UniverseMutationError> {
    let parsed = mutations
        .iter()
        .enumerate()
        .map(|(index, mutation)| validate_mutation(index, mutation))
        .collect::<Result<Vec<_>, _>>()?;

    let mut next = state.clone();
    for mutation in parsed {
        apply_valid_mutation(&mut next, mutation, recorded_at);
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
    SetNumeric {
        key: String,
        value: f64,
    },
    ClearNumeric {
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

/// A mutation that passed validation: what to change, the target it changes as
/// the mutation spelled it, and the provenance to record if it writes.
#[derive(Debug, Clone, PartialEq)]
struct ValidatedMutation {
    target: String,
    kind: ProvenanceKind,
    change: ValidMutation,
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
) -> Result<ValidatedMutation, UniverseMutationError> {
    Ok(ValidatedMutation {
        target: mutation.target.clone(),
        kind: mutation.provenance_kind.unwrap_or_default(),
        change: validate_change(index, mutation)?,
    })
}

fn validate_change(
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
            require_nothing_to_write(index, mutation, "clear_fact")?;
            Ok(ValidMutation::ClearFact { key: target.key })
        }
        "set_numeric" => {
            require_collection(index, target.collection, TargetCollection::NumericValues)?;
            Ok(ValidMutation::SetNumeric {
                key: target.key,
                value: require_numeric_payload(index, mutation)?,
            })
        }
        "clear_numeric" => {
            require_collection(index, target.collection, TargetCollection::NumericValues)?;
            require_nothing_to_write(index, mutation, "clear_numeric")?;
            Ok(ValidMutation::ClearNumeric { key: target.key })
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

/// What a change did to the value at its target, which decides what happens to
/// that target's provenance.
enum Outcome {
    /// The value was written: its provenance is recorded.
    Written,
    /// The value is gone: its provenance goes with it.
    Removed,
    /// Nothing changed, so neither does the provenance.
    Untouched,
}

fn apply_valid_mutation(
    state: &mut UniverseState,
    mutation: ValidatedMutation,
    recorded_at: UbuTimestamp,
) {
    let ValidatedMutation {
        target,
        kind,
        change,
    } = mutation;
    let outcome = match change {
        ValidMutation::SetFact { key, value } => {
            state.facts.insert(key, value);
            Outcome::Written
        }
        ValidMutation::ClearFact { key } => {
            state.facts.remove(&key);
            Outcome::Removed
        }
        ValidMutation::SetNumeric { key, value } => {
            state.numeric_values.insert(key, value);
            Outcome::Written
        }
        ValidMutation::ClearNumeric { key } => {
            state.numeric_values.remove(&key);
            Outcome::Removed
        }
        ValidMutation::IncrementNumeric { key, delta } => {
            *state.numeric_values.entry(key).or_insert(0.0) += delta;
            Outcome::Written
        }
        ValidMutation::DecrementNumeric { key, delta } => {
            *state.numeric_values.entry(key).or_insert(0.0) -= delta;
            Outcome::Written
        }
        ValidMutation::AddMembership { key, member } => {
            state.set_memberships.entry(key).or_default().insert(member);
            Outcome::Written
        }
        // A set exists only while it has a member. Removing the last one removes
        // the set, and a set that was never there is as gone as one that was.
        ValidMutation::RemoveMembership { key, member } => {
            match state.set_memberships.get_mut(&key) {
                Some(members) => {
                    let removed = members.remove(&member);
                    if members.is_empty() {
                        state.set_memberships.remove(&key);
                        Outcome::Removed
                    } else if removed {
                        Outcome::Written
                    } else {
                        Outcome::Untouched
                    }
                }
                None => Outcome::Removed,
            }
        }
        ValidMutation::AppendEventMarker { key, marker } => {
            state.event_markers.entry(key).or_default().push(marker);
            Outcome::Written
        }
    };
    match outcome {
        Outcome::Written => {
            state
                .fact_provenance
                .insert(target, FactProvenance { kind, recorded_at });
        }
        Outcome::Removed => {
            state.fact_provenance.remove(&target);
        }
        Outcome::Untouched => {}
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
        // The four numeric comparisons, in the shape of `member_of`: the target's
        // collection, then `expected`, then the comparison. A number that was
        // never recorded satisfies none of them, and it is not malformed to ask.
        predicate @ ("at_least" | "at_most" | "greater_than" | "less_than") => {
            if target.collection != TargetCollection::NumericValues {
                return Err(UniversePreconditionError::Malformed(format!(
                    "{predicate} requires a numeric_values target"
                )));
            }
            let expected = leaf.expected.as_ref().ok_or_else(|| {
                UniversePreconditionError::Malformed(format!("{predicate} requires expected"))
            })?;
            // A comparison against a value that is not finite is false whatever
            // the number is, so it is refused and not silently never satisfied.
            let expected = expected
                .as_f64()
                .filter(|expected| expected.is_finite())
                .ok_or_else(|| {
                    UniversePreconditionError::Malformed(format!(
                        "{predicate} expected value must be a finite number"
                    ))
                })?;
            Ok(state
                .numeric_values
                .get(&target.key)
                .is_some_and(|actual| match predicate {
                    "at_least" => *actual >= expected,
                    "at_most" => *actual <= expected,
                    "greater_than" => *actual > expected,
                    _ => *actual < expected,
                }))
        }
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

/// A clear writes nothing, so it takes neither a payload nor a provenance kind
/// to describe one.
fn require_nothing_to_write(
    index: usize,
    mutation: &UniverseMutation,
    operation: &str,
) -> Result<(), UniverseMutationError> {
    if mutation.payload.is_some() {
        return Err(invalid_mutation(
            index,
            format!("{operation} does not accept a payload"),
        ));
    }
    if mutation.provenance_kind.is_some() {
        return Err(invalid_mutation(
            index,
            format!("{operation} does not accept a provenance kind"),
        ));
    }
    Ok(())
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
            provenance_kind: None,
        }
    }

    fn written_at() -> UbuTimestamp {
        UbuTimestamp::parse("2026-06-22T12:30:00Z").expect("valid timestamp")
    }

    /// The applicator at one fixed write time, which the older tests do not read.
    fn apply(
        state: &UniverseState,
        mutations: &[UniverseMutation],
    ) -> Result<UniverseState, UniverseMutationError> {
        apply_universe_mutations(state, mutations, written_at())
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
        let next = apply(
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
        let initial = apply(
            &state(),
            &[mutation(
                "set_fact",
                "facts.ticket.status",
                Some(json!("ready")),
            )],
        )
        .expect("valid mutation");

        let next = apply(
            &initial,
            &[mutation("clear_fact", "facts.ticket.status", None)],
        )
        .expect("valid mutation");

        assert!(!next.facts.contains_key("ticket.status"));
    }

    #[test]
    fn increment_numeric_initializes_missing_key_from_zero() {
        let next = apply(
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
        let next = apply(
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
        let next = apply(
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
        let initial = apply(
            &state(),
            &[mutation(
                "add_membership",
                "set_memberships.tags",
                Some(json!("focused")),
            )],
        )
        .expect("valid mutation");

        let next = apply(
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
        let next = apply(
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
        let result = apply(
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
        let next = apply(
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
        let next = apply(
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
        let next = apply(
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
        let initial = apply(
            &state(),
            &[mutation(
                "set_fact",
                "facts.ticket.status",
                Some(json!("ready")),
            )],
        )
        .expect("valid mutation");
        let next = apply(
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
        let next = apply(
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
        let next = apply(
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
    fn member_of_is_malformed_on_a_numeric_target() {
        let next = apply(
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
            &leaf("facts.status", "roughly_equals", Some(json!(1)))
        )
        .is_err());
        assert!(
            evaluate_universe_precondition(&state(), &leaf("facts.status", "equals", None))
                .is_err()
        );
    }

    #[test]
    fn is_intrinsic_affect_target_uses_namespace_predicate() {
        assert!(is_intrinsic_affect_target("numeric_values.affect.energy"));
        assert!(is_intrinsic_affect_target("facts.affect.recent_state"));
        // `affect` as the collection (first segment) is not the namespace.
        assert!(!is_intrinsic_affect_target("affect.energy"));
        assert!(!is_intrinsic_affect_target("facts.ticket.status"));
        assert!(!is_intrinsic_affect_target("facts"));
    }

    #[test]
    fn user_mode_permits_affect_targets_in_precondition_and_mutations() {
        let precondition = UniversePrecondition::AllOf {
            all_of: vec![
                leaf("numeric_values.affect.energy", "equals", Some(json!(1.0))),
                leaf("facts.ticket.status", "equals", Some(json!("ready"))),
            ],
        };
        let mutations = vec![mutation(
            "increment_numeric",
            "numeric_values.affect.energy",
            Some(json!(1)),
        )];

        assert_eq!(
            validate_precondition_for_mode(InstanceMode::UserMode, &precondition),
            Ok(())
        );
        assert_eq!(
            validate_mutations_for_mode(InstanceMode::UserMode, &mutations),
            Ok(())
        );
    }

    #[test]
    fn organization_and_worker_mode_reject_affect_targets() {
        let precondition = UniversePrecondition::AnyOf {
            any_of: vec![
                leaf("facts.ticket.status", "equals", Some(json!("ready"))),
                leaf("facts.affect.recent_state", "equals", Some(json!("calm"))),
            ],
        };
        let mutations = vec![mutation(
            "set_fact",
            "facts.affect.recent_state",
            Some(json!("calm")),
        )];

        for mode in [InstanceMode::OrganizationMode, InstanceMode::WorkerMode] {
            assert_eq!(
                validate_precondition_for_mode(mode, &precondition),
                Err(ModeValidationError::IntrinsicAffectForbidden {
                    mode,
                    target: "facts.affect.recent_state".to_owned(),
                })
            );
            assert_eq!(
                validate_mutations_for_mode(mode, &mutations),
                Err(ModeValidationError::IntrinsicAffectForbidden {
                    mode,
                    target: "facts.affect.recent_state".to_owned(),
                })
            );
        }
    }

    #[test]
    fn non_affect_targets_pass_in_all_modes() {
        let precondition = leaf("facts.ticket.status", "equals", Some(json!("ready")));
        let mutations = vec![mutation(
            "set_fact",
            "facts.ticket.status",
            Some(json!("ready")),
        )];

        for mode in [
            InstanceMode::UserMode,
            InstanceMode::OrganizationMode,
            InstanceMode::WorkerMode,
        ] {
            assert_eq!(validate_precondition_for_mode(mode, &precondition), Ok(()));
            assert_eq!(validate_mutations_for_mode(mode, &mutations), Ok(()));
        }
    }

    #[test]
    fn instance_mode_serde_matches_section_five_strings() {
        assert_eq!(
            serde_json::to_value(InstanceMode::UserMode).unwrap(),
            json!("user_mode")
        );
        assert_eq!(
            serde_json::to_value(InstanceMode::OrganizationMode).unwrap(),
            json!("organization_mode")
        );
        assert_eq!(
            serde_json::to_value(InstanceMode::WorkerMode).unwrap(),
            json!("worker_mode")
        );
        assert_eq!(
            serde_json::from_value::<InstanceMode>(json!("worker_mode")).unwrap(),
            InstanceMode::WorkerMode
        );
    }

    #[test]
    fn mutation_validation_rejects_wrong_payload_types() {
        assert!(apply(
            &state(),
            &[mutation(
                "add_membership",
                "set_memberships.tags",
                Some(json!(["not", "scalar"]))
            )]
        )
        .is_err());
        assert!(apply(
            &state(),
            &[mutation(
                "append_event_marker",
                "event_markers.audit",
                Some(json!("not object"))
            )]
        )
        .is_err());
        assert!(apply(&state(), &[mutation("set_fact", "facts.status", None)]).is_err());
    }

    // ---- P1B-59: a measured number is a first-class fact. Every value is invented.

    fn kinded(
        operation: &str,
        target: &str,
        payload: Option<Value>,
        kind: ProvenanceKind,
    ) -> UniverseMutation {
        UniverseMutation {
            provenance_kind: Some(kind),
            ..mutation(operation, target, payload)
        }
    }

    fn refusal(state: &UniverseState, mutation: UniverseMutation) -> String {
        apply(state, &[mutation])
            .expect_err("the mutation is refused")
            .to_string()
    }

    /// The one failure this design can have: an entry for a value that is gone.
    fn assert_no_stale_provenance(state: &UniverseState) {
        for target in state.fact_provenance.keys() {
            let parsed = parse_target(target).expect("a provenance key is a target");
            assert!(
                value_at_target(state, &parsed).is_some(),
                "provenance for `{target}` outlived its value"
            );
        }
    }

    #[test]
    fn set_numeric_replaces_whatever_is_there() {
        let jars = "numeric_values.shelf.jars";
        let first = apply(&state(), &[mutation("set_numeric", jars, Some(json!(0.7)))]).unwrap();
        assert_eq!(first.numeric_values.get("shelf.jars"), Some(&0.7));

        // A reading is set, not reached by a difference: from 0.7, 0.1 is 0.1.
        let second = apply(&first, &[mutation("set_numeric", jars, Some(json!(0.1)))]).unwrap();
        assert_eq!(second.numeric_values.get("shelf.jars"), Some(&0.1));
        assert_ne!(
            0.7 - (0.7 - 0.1),
            0.1,
            "the difference would not have landed"
        );

        // A tally is still a tally: increment and decrement count from what is set.
        let third = apply(
            &second,
            &[
                mutation("set_numeric", jars, Some(json!(4))),
                mutation("increment_numeric", jars, Some(json!(2))),
                mutation("decrement_numeric", jars, Some(json!(1))),
            ],
        )
        .unwrap();
        assert_eq!(third.numeric_values.get("shelf.jars"), Some(&5.0));
    }

    #[test]
    fn clear_numeric_removes_the_key_and_is_a_no_op_on_an_absent_key() {
        let jars = "numeric_values.shelf.jars";
        let empty = state();
        let set = apply(&empty, &[mutation("set_numeric", jars, Some(json!(3)))]).unwrap();
        let cleared = apply(&set, &[mutation("clear_numeric", jars, None)]).unwrap();
        assert!(!cleared.numeric_values.contains_key("shelf.jars"));
        assert_eq!(cleared, empty, "a set and a clear leave nothing behind");

        // Clearing what is not there is not an error, and changes nothing.
        assert_eq!(
            apply(&empty, &[mutation("clear_numeric", jars, None)]),
            Ok(empty)
        );
    }

    #[test]
    fn set_numeric_and_clear_numeric_refuse_their_malformed_forms() {
        let jars = "numeric_values.shelf.jars";
        for (bad, message) in [
            (
                mutation("set_numeric", jars, None),
                "operation requires a payload",
            ),
            (
                mutation("set_numeric", jars, Some(json!("three"))),
                "payload must be a JSON number",
            ),
            (
                mutation("set_numeric", jars, Some(json!(true))),
                "payload must be a JSON number",
            ),
            (
                mutation("set_numeric", jars, Some(json!([3]))),
                "payload must be a JSON number",
            ),
            (
                mutation("set_numeric", "facts.shelf.jars", Some(json!(3))),
                "operation target must be in the numeric_values collection",
            ),
            (
                mutation("set_numeric", "numeric_values..jars", Some(json!(3))),
                "malformed target `numeric_values..jars`",
            ),
            (
                mutation("clear_numeric", jars, Some(json!(0))),
                "clear_numeric does not accept a payload",
            ),
            (
                mutation("clear_numeric", "facts.shelf.jars", None),
                "operation target must be in the numeric_values collection",
            ),
            (
                kinded("clear_numeric", jars, None, ProvenanceKind::Measured),
                "clear_numeric does not accept a provenance kind",
            ),
            (
                kinded(
                    "clear_fact",
                    "facts.shelf.label",
                    None,
                    ProvenanceKind::Asserted,
                ),
                "clear_fact does not accept a provenance kind",
            ),
            (
                mutation("clear_fact", "facts.shelf.label", Some(json!(1))),
                "clear_fact does not accept a payload",
            ),
        ] {
            assert_eq!(
                refusal(&state(), bad.clone()),
                format!("mutation 0: {message}"),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn each_comparison_is_true_and_false_where_it_should_be() {
        let level = "numeric_values.tank.level";
        let state = apply(&state(), &[mutation("set_numeric", level, Some(json!(25)))]).unwrap();
        for (predicate, expected, holds) in [
            ("at_least", 24.5, true),
            ("at_least", 25.0, true),
            ("at_least", 25.5, false),
            ("at_most", 25.5, true),
            ("at_most", 25.0, true),
            ("at_most", 24.5, false),
            ("greater_than", 24.5, true),
            ("greater_than", 25.0, false),
            ("greater_than", 25.5, false),
            ("less_than", 25.5, true),
            ("less_than", 25.0, false),
            ("less_than", 24.5, false),
        ] {
            assert_eq!(
                evaluate_universe_precondition(
                    &state,
                    &leaf(level, predicate, Some(json!(expected)))
                ),
                Ok(holds),
                "25 {predicate} {expected}"
            );
        }
        // An integer `expected` compares as the number it is.
        assert_eq!(
            evaluate_universe_precondition(&state, &leaf(level, "at_least", Some(json!(25)))),
            Ok(true)
        );
        // Negative numbers and zero are numbers like any other.
        let cold = apply(&state, &[mutation("set_numeric", level, Some(json!(-3)))]).unwrap();
        assert_eq!(
            evaluate_universe_precondition(&cold, &leaf(level, "less_than", Some(json!(0)))),
            Ok(true)
        );
    }

    #[test]
    fn a_number_that_was_never_recorded_satisfies_no_comparison_and_is_not_an_error() {
        for predicate in ["at_least", "at_most", "greater_than", "less_than"] {
            assert_eq!(
                evaluate_universe_precondition(
                    &state(),
                    &leaf("numeric_values.tank.level", predicate, Some(json!(25)))
                ),
                Ok(false),
                "{predicate} on an absent key"
            );
        }
        // And one that was recorded and then cleared is absent again.
        let level = "numeric_values.tank.level";
        let cleared = apply(
            &state(),
            &[
                mutation("set_numeric", level, Some(json!(40))),
                mutation("clear_numeric", level, None),
            ],
        )
        .unwrap();
        assert_eq!(
            evaluate_universe_precondition(&cleared, &leaf(level, "at_least", Some(json!(25)))),
            Ok(false)
        );
    }

    #[test]
    fn a_comparison_is_malformed_off_numeric_values_or_without_a_finite_number() {
        let level = "numeric_values.tank.level";
        let state = apply(&state(), &[mutation("set_numeric", level, Some(json!(25)))]).unwrap();
        for predicate in ["at_least", "at_most", "greater_than", "less_than"] {
            let malformed =
                |target: &str, expected: Option<Value>| match evaluate_universe_precondition(
                    &state,
                    &leaf(target, predicate, expected),
                ) {
                    Err(UniversePreconditionError::Malformed(message)) => message,
                    other => panic!("{predicate} on {target}: expected Malformed, got {other:?}"),
                };
            for target in [
                "facts.tank.level",
                "set_memberships.tank.level",
                "event_markers.tank.level",
            ] {
                assert_eq!(
                    malformed(target, Some(json!(25))),
                    format!("{predicate} requires a numeric_values target")
                );
            }
            assert_eq!(
                malformed(level, None),
                format!("{predicate} requires expected")
            );
            // serde_json has no NaN and no infinity: `Value::from` turns both into
            // null. So a non-finite `expected` arrives as not a number at all.
            assert_eq!(Value::from(f64::NAN), Value::Null);
            for expected in [
                Value::from(f64::NAN),
                Value::from(f64::INFINITY),
                Value::from(f64::NEG_INFINITY),
                json!("25"),
                json!("NaN"),
                json!(true),
                json!([25]),
                json!({"value": 25}),
            ] {
                assert_eq!(
                    malformed(level, Some(expected.clone())),
                    format!("{predicate} expected value must be a finite number"),
                    "{expected}"
                );
            }
        }
    }

    #[test]
    fn a_write_records_its_provenance_and_no_stated_kind_is_asserted() {
        let next = apply(
            &state(),
            &[
                mutation("set_fact", "facts.kettle.descaled", Some(json!(true))),
                kinded(
                    "set_numeric",
                    "numeric_values.tank.level",
                    Some(json!(25)),
                    ProvenanceKind::Measured,
                ),
                kinded(
                    "increment_numeric",
                    "numeric_values.shelf.jars",
                    Some(json!(2)),
                    ProvenanceKind::Derived,
                ),
                kinded(
                    "add_membership",
                    "set_memberships.toolbox",
                    Some(json!("spanner")),
                    ProvenanceKind::Proposed,
                ),
                mutation(
                    "append_event_marker",
                    "event_markers.kettle.boiled",
                    Some(json!({"cups": 2})),
                ),
            ],
        )
        .unwrap();
        let entry = |kind| FactProvenance {
            kind,
            recorded_at: written_at(),
        };
        assert_eq!(
            next.fact_provenance,
            BTreeMap::from([
                (
                    "facts.kettle.descaled".to_owned(),
                    entry(ProvenanceKind::Asserted)
                ),
                (
                    "numeric_values.tank.level".to_owned(),
                    entry(ProvenanceKind::Measured)
                ),
                (
                    "numeric_values.shelf.jars".to_owned(),
                    entry(ProvenanceKind::Derived)
                ),
                (
                    "set_memberships.toolbox".to_owned(),
                    entry(ProvenanceKind::Proposed)
                ),
                (
                    "event_markers.kettle.boiled".to_owned(),
                    entry(ProvenanceKind::Asserted)
                ),
            ])
        );
        // The write time is the caller's, and the state's own time does not move.
        assert_eq!(next.captured_at, state().captured_at);
        assert_ne!(next.captured_at, written_at());
        assert_no_stale_provenance(&next);

        // A later write replaces the entry: the kind and the time are the last write's.
        let later = UbuTimestamp::parse("2026-06-23T08:00:00Z").unwrap();
        let again = apply_universe_mutations(
            &next,
            &[mutation(
                "set_numeric",
                "numeric_values.tank.level",
                Some(json!(18)),
            )],
            later,
        )
        .unwrap();
        assert_eq!(
            again.fact_provenance["numeric_values.tank.level"],
            FactProvenance {
                kind: ProvenanceKind::Asserted,
                recorded_at: later
            }
        );
        assert_eq!(
            again.fact_provenance["facts.kettle.descaled"],
            entry(ProvenanceKind::Asserted)
        );
    }

    #[test]
    fn no_provenance_entry_survives_the_removal_of_its_value() {
        let measured = ProvenanceKind::Measured;
        let empty = state();
        let written = apply(
            &empty,
            &[
                kinded(
                    "set_fact",
                    "facts.kettle.descaled",
                    Some(json!(true)),
                    measured,
                ),
                kinded(
                    "set_numeric",
                    "numeric_values.tank.level",
                    Some(json!(25)),
                    measured,
                ),
                kinded(
                    "increment_numeric",
                    "numeric_values.shelf.jars",
                    Some(json!(2)),
                    measured,
                ),
                kinded(
                    "add_membership",
                    "set_memberships.toolbox",
                    Some(json!("spanner")),
                    measured,
                ),
                kinded(
                    "add_membership",
                    "set_memberships.toolbox",
                    Some(json!("chisel")),
                    measured,
                ),
            ],
        )
        .unwrap();
        assert_eq!(written.fact_provenance.len(), 4);
        assert_no_stale_provenance(&written);

        // Each removal, one at a time, takes the entry with the value.
        let mut current = written.clone();
        for (remove, gone) in [
            (
                mutation("clear_fact", "facts.kettle.descaled", None),
                "facts.kettle.descaled",
            ),
            (
                mutation("clear_numeric", "numeric_values.tank.level", None),
                "numeric_values.tank.level",
            ),
            (
                mutation("clear_numeric", "numeric_values.shelf.jars", None),
                "numeric_values.shelf.jars",
            ),
        ] {
            current = apply(&current, &[remove]).unwrap();
            assert!(!current.fact_provenance.contains_key(gone), "{gone}");
            assert_no_stale_provenance(&current);
        }

        // A set keeps its entry while it has a member, and loses it with the last one.
        let toolbox = "set_memberships.toolbox";
        current = apply(
            &current,
            &[mutation(
                "remove_membership",
                toolbox,
                Some(json!("mallet")),
            )],
        )
        .unwrap();
        assert_eq!(
            current.fact_provenance[toolbox].kind, measured,
            "removing a non-member touches nothing"
        );
        current = apply(
            &current,
            &[mutation(
                "remove_membership",
                toolbox,
                Some(json!("spanner")),
            )],
        )
        .unwrap();
        assert_eq!(
            current.fact_provenance[toolbox].kind,
            ProvenanceKind::Asserted,
            "the set was rewritten, on someone's word"
        );
        assert_no_stale_provenance(&current);
        current = apply(
            &current,
            &[mutation(
                "remove_membership",
                toolbox,
                Some(json!("chisel")),
            )],
        )
        .unwrap();
        assert!(current.set_memberships.is_empty());
        assert!(
            current.fact_provenance.is_empty(),
            "{:?}",
            current.fact_provenance
        );
        assert_eq!(
            current, empty,
            "everything written was removed, and nothing is left of it"
        );

        // Written and removed in one list: the entry does not outlive the list.
        let both = apply(
            &empty,
            &[
                kinded(
                    "set_numeric",
                    "numeric_values.tank.level",
                    Some(json!(25)),
                    measured,
                ),
                mutation("clear_numeric", "numeric_values.tank.level", None),
                kinded(
                    "set_fact",
                    "facts.kettle.descaled",
                    Some(json!(true)),
                    measured,
                ),
                mutation("clear_fact", "facts.kettle.descaled", None),
                kinded("add_membership", toolbox, Some(json!("spanner")), measured),
                mutation("remove_membership", toolbox, Some(json!("spanner"))),
            ],
        )
        .unwrap();
        assert_eq!(both, empty);

        // A removal of what was never there leaves no entry either, even one put there by hand.
        let mut orphaned = state();
        for target in [
            "facts.kettle.descaled",
            "numeric_values.tank.level",
            toolbox,
        ] {
            orphaned.fact_provenance.insert(
                target.to_owned(),
                FactProvenance {
                    kind: measured,
                    recorded_at: written_at(),
                },
            );
        }
        let swept = apply(
            &orphaned,
            &[
                mutation("clear_fact", "facts.kettle.descaled", None),
                mutation("clear_numeric", "numeric_values.tank.level", None),
                mutation("remove_membership", toolbox, Some(json!("spanner"))),
            ],
        )
        .unwrap();
        assert!(swept.fact_provenance.is_empty());
    }

    #[test]
    fn a_refused_list_records_no_provenance() {
        let result = apply(
            &state(),
            &[
                kinded(
                    "set_numeric",
                    "numeric_values.tank.level",
                    Some(json!(25)),
                    ProvenanceKind::Measured,
                ),
                mutation(
                    "set_numeric",
                    "numeric_values.tank.level",
                    Some(json!("full")),
                ),
            ],
        );
        assert_eq!(
            result.unwrap_err().to_string(),
            "mutation 1: payload must be a JSON number"
        );
    }

    #[test]
    fn fact_provenance_is_a_sibling_of_the_value_maps_on_the_wire() {
        // With no entry the key is absent, so a state from before this field
        // serializes as it always did.
        let plain = serde_json::to_value(state()).unwrap();
        assert!(plain.get("fact_provenance").is_none(), "{plain}");

        let written = apply(
            &state(),
            &[kinded(
                "set_numeric",
                "numeric_values.tank.level",
                Some(json!(25)),
                ProvenanceKind::Measured,
            )],
        )
        .unwrap();
        let wire = serde_json::to_value(&written).unwrap();
        assert_eq!(wire["numeric_values"], json!({"tank.level": 25.0}));
        assert_eq!(
            wire["fact_provenance"],
            json!({"numeric_values.tank.level": {"kind": "measured", "recorded_at": "2026-06-22T12:30:00Z"}})
        );
        assert_eq!(
            serde_json::from_value::<UniverseState>(wire).unwrap(),
            written
        );
    }

    #[test]
    fn a_stored_payload_keeps_its_envelope_under_provenance_beside_fact_provenance() {
        // What the store holds: the state, the object envelope under `provenance`,
        // and a schema version. The envelope is not the per-fact map, and reading
        // the row must not mistake one for the other.
        let stored = json!({
            "id": state().id,
            "captured_at": "2026-06-22T12:00:00Z",
            "numeric_values": {"tank.level": 25.0},
            "fact_provenance": {
                "numeric_values.tank.level": {"kind": "measured", "recorded_at": "2026-06-22T12:30:00Z"}
            },
            "source_summary": "test fixture",
            "schema_version": "core/universe-state/0.1",
            "provenance": {"created_at": "2026-06-22T12:00:00Z", "authority_source": "user"}
        });
        let read: UniverseState =
            serde_json::from_value(stored).expect("a stored row deserializes");
        assert_eq!(
            read.fact_provenance["numeric_values.tank.level"],
            FactProvenance {
                kind: ProvenanceKind::Measured,
                recorded_at: written_at()
            }
        );
        assert_eq!(read.fact_provenance.len(), 1);
    }

    #[test]
    fn a_provenance_entry_is_a_kind_and_a_time_and_nothing_else() {
        let entry = json!({"kind": "derived", "recorded_at": "2026-06-22T12:30:00Z"});
        assert_eq!(
            serde_json::from_value::<FactProvenance>(entry.clone()).unwrap(),
            FactProvenance {
                kind: ProvenanceKind::Derived,
                recorded_at: written_at()
            }
        );
        for (extra, value) in [
            ("confidence", json!(0.9)),
            ("note", json!("read off the invented dial")),
        ] {
            let mut with_extra = entry.clone();
            with_extra[extra] = value;
            assert!(
                serde_json::from_value::<FactProvenance>(with_extra).is_err(),
                "{extra}"
            );
        }
        assert!(serde_json::from_value::<FactProvenance>(
            json!({"kind": "guessed", "recorded_at": "2026-06-22T12:30:00Z"})
        )
        .is_err());
        assert!(serde_json::from_value::<FactProvenance>(json!({"kind": "measured"})).is_err());
        for kind in [
            ProvenanceKind::Asserted,
            ProvenanceKind::Measured,
            ProvenanceKind::Derived,
            ProvenanceKind::Proposed,
        ] {
            assert_eq!(serde_json::to_value(kind).unwrap(), json!(kind.as_str()));
        }
        assert_eq!(ProvenanceKind::default(), ProvenanceKind::Asserted);
    }

    #[test]
    fn a_mutation_has_no_note_and_states_its_kind_only_when_it_has_one() {
        let plain = mutation("set_numeric", "numeric_values.tank.level", Some(json!(25)));
        assert_eq!(
            serde_json::to_value(&plain).unwrap(),
            json!({"operation": "set_numeric", "target": "numeric_values.tank.level", "payload": 25})
        );
        let measured = json!({"operation": "set_numeric", "target": "numeric_values.tank.level", "payload": 25, "provenance_kind": "measured"});
        let parsed: UniverseMutation = serde_json::from_value(measured.clone()).unwrap();
        assert_eq!(parsed.provenance_kind, Some(ProvenanceKind::Measured));
        assert_eq!(serde_json::to_value(&parsed).unwrap(), measured);

        // `note` was accepted and stored nowhere. It is refused, not dropped.
        let mut noted = measured.clone();
        noted["note"] = json!("read off the invented dial");
        assert!(serde_json::from_value::<UniverseMutation>(noted).is_err());
        let mut guessed = measured;
        guessed["provenance_kind"] = json!("guessed");
        assert!(serde_json::from_value::<UniverseMutation>(guessed).is_err());
    }

    #[test]
    fn the_mode_guard_reads_the_new_operations_and_predicates_like_any_other() {
        let mutations = vec![mutation(
            "set_numeric",
            "numeric_values.affect.energy",
            Some(json!(1)),
        )];
        let precondition = leaf("numeric_values.affect.energy", "at_least", Some(json!(1)));
        assert_eq!(
            validate_mutations_for_mode(InstanceMode::UserMode, &mutations),
            Ok(())
        );
        for mode in [InstanceMode::OrganizationMode, InstanceMode::WorkerMode] {
            let forbidden = Err(ModeValidationError::IntrinsicAffectForbidden {
                mode,
                target: "numeric_values.affect.energy".to_owned(),
            });
            assert_eq!(validate_mutations_for_mode(mode, &mutations), forbidden);
            assert_eq!(
                validate_precondition_for_mode(mode, &precondition),
                forbidden
            );
        }
    }

    #[test]
    fn a_key_that_repeats_its_collection_hides_its_namespace_from_the_mode_guard() {
        // Why a key must not carry its collection. The guard reads the segment
        // after the collection as the namespace. A fact stored under the key
        // `facts.affect.mood`, inside `facts`, is addressed `facts.facts.affect.mood`,
        // whose namespace reads as `facts`: the guard does not fire.
        assert!(is_intrinsic_affect_target("facts.affect.mood"));
        assert!(!is_intrinsic_affect_target("facts.facts.affect.mood"));
        assert!(!is_intrinsic_affect_target(
            "numeric_values.numeric_values.affect.energy"
        ));
    }
}
