use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use ubu_core::core::{ExternalReference, LogEntry, Objective, Snapshot, Task};
use ubu_core::planning::{
    PlanningRequest, PlanningResponse, RepairRequest, RepairResponse,
    PLANNING_KERNEL_CONTRACT_VERSION,
};
use ubu_core::policy_summary::PolicySummary;
use ubu_core::projection::ProjectionPreview;
use ubu_core::store::RecalculationTrigger;
use ubu_core::worker::{GpuAdvisoryRequest, GpuAdvisoryResponse};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("UBU_SCHEMAS_FIXTURES"))
}

fn assert_submodule_state_is_explicit() {
    let present = env!("UBU_SCHEMAS_REF_PRESENT");
    assert!(
        present == "0" || present == "1",
        "UBU_SCHEMAS_REF_PRESENT must be explicit"
    );
}

fn round_trip_fixture<T>(relative: &str)
where
    T: Serialize + DeserializeOwned + std::fmt::Debug,
{
    let path = fixture_root().join(relative);
    let json = fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("failed to read fixture {}: {err}", path.display());
    });

    let original: Value = serde_json::from_str(&json).expect("valid fixture json");
    let parsed: T = serde_json::from_value(original.clone()).expect("fixture deserializes");
    let serialized = serde_json::to_value(parsed).expect("fixture serializes");
    assert_eq!(serialized, original);
}

fn assert_fixture_rejected<T>(relative: &str)
where
    T: DeserializeOwned + std::fmt::Debug,
{
    let path = fixture_root().join(relative);
    let json = fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("failed to read fixture {}: {err}", path.display());
    });

    let original: Value = serde_json::from_str(&json).expect("valid fixture json");
    serde_json::from_value::<T>(original).expect_err("fixture should not deserialize");
}

#[test]
fn fixture_root_exists() {
    assert_submodule_state_is_explicit();
    assert!(
        Path::new(env!("UBU_SCHEMAS_FIXTURES")).is_dir(),
        "fixture root should exist"
    );
}

#[test]
fn round_trips_canonical_or_placeholder_fixtures() {
    assert_eq!(
        PLANNING_KERNEL_CONTRACT_VERSION,
        "planning-kernel-contract/0.1"
    );

    round_trip_fixture::<Task>("valid/core/task/basic.json");
    round_trip_fixture::<Objective>("valid/core/objective/basic.json");
    round_trip_fixture::<ExternalReference>("valid/core/external-reference/basic.json");
    round_trip_fixture::<LogEntry>("valid/core/log-entry/basic.json");
    round_trip_fixture::<LogEntry>("valid/core/log-entry/compartment-boundary-decided.json");
    round_trip_fixture::<Snapshot>("valid/core/snapshot/bootstrap-defaults.json");
    round_trip_fixture::<Snapshot>("valid/core/snapshot/live-observation.json");
    round_trip_fixture::<PolicySummary>("valid/common/policy-summary/guardrail-members.json");
    round_trip_fixture::<PlanningRequest>("valid/planning/planning-request/basic.json");
    round_trip_fixture::<PlanningResponse>("valid/planning/planning-response/basic.json");
    round_trip_fixture::<RepairRequest>("valid/planning/repair-request/basic.json");
    round_trip_fixture::<RepairResponse>("valid/planning/repair-response/basic.json");
    round_trip_fixture::<RecalculationTrigger>("valid/store/recalculation-trigger/basic.json");
    round_trip_fixture::<ProjectionPreview>("valid/projection/projection-preview/basic.json");
    round_trip_fixture::<GpuAdvisoryRequest>("valid/worker/gpu-advisory-request/basic.json");
    round_trip_fixture::<GpuAdvisoryResponse>("valid/worker/gpu-advisory-response/basic.json");
}

#[test]
fn rejects_free_form_recalculation_reason_fixture() {
    assert_fixture_rejected::<RecalculationTrigger>(
        "invalid/store/recalculation-trigger/old-reason.json",
    );
}
