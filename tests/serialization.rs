use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use ubu_core::core::{ExternalReference, LogEntry, Objective, Snapshot, Task};
use ubu_core::planning::{PlanningRequest, PlanningResponse, RepairRequest, RepairResponse};
use ubu_core::policy_summary::PolicySummary;
use ubu_core::projection::ProjectionPreview;
use ubu_core::store::RecalculationTrigger;

/// Resolve a fixture against the canonical `ubu-schemas` fixtures first, falling
/// back to the `ubu-core`-owned placeholders for the planning/repair contract
/// types whose canonical fixtures were removed from `ubu-schemas`.
fn resolve_fixture(relative: &str) -> PathBuf {
    let canonical = PathBuf::from(env!("UBU_SCHEMAS_FIXTURES")).join(relative);
    if canonical.is_file() {
        canonical
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/placeholders")
            .join(relative)
    }
}

fn fixture(relative: &str) -> String {
    let path = resolve_fixture(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn round_trip<T>(relative: &str)
where
    T: Serialize + DeserializeOwned + std::fmt::Debug,
{
    let json = fixture(relative);
    let original: Value = serde_json::from_str(&json).expect("valid json");
    let parsed: T = serde_json::from_value(original.clone()).expect("deserialize");
    let serialized = serde_json::to_value(parsed).expect("serialize");
    assert_eq!(serialized, original);
}

#[test]
fn serde_round_trips_core_and_planning_types() {
    round_trip::<Task>("valid/core/task/basic.json");
    round_trip::<Objective>("valid/core/objective/basic.json");
    round_trip::<ExternalReference>("valid/core/external-reference/basic.json");
    round_trip::<LogEntry>("valid/core/log-entry/basic.json");
    round_trip::<LogEntry>("valid/core/log-entry/compartment-boundary-decided.json");
    round_trip::<Snapshot>("valid/core/snapshot/bootstrap-defaults.json");
    round_trip::<Snapshot>("valid/core/snapshot/live-observation.json");
    round_trip::<PolicySummary>("valid/common/policy-summary/guardrail-members.json");
    round_trip::<PlanningRequest>("valid/planning/planning-request/basic.json");
    round_trip::<PlanningResponse>("valid/planning/planning-response/basic.json");
    round_trip::<RepairRequest>("valid/planning/repair-request/basic.json");
    round_trip::<RepairResponse>("valid/planning/repair-response/basic.json");
    round_trip::<RecalculationTrigger>("valid/store/recalculation-trigger/basic.json");
    round_trip::<ProjectionPreview>("valid/projection/projection-preview/basic.json");
}
