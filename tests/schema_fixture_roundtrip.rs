use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use ubu_core::core::{ExternalReference, LogEntry, Objective, Snapshot, Task};
use ubu_core::{AdvisoryCandidate, DeviceRegistration, SuppressionRecord};
use ubu_core::planning::{
    PlanningRequest, PlanningResponse, RepairRequest, RepairResponse,
    PLANNING_KERNEL_CONTRACT_VERSION,
};
use ubu_core::policy_summary::PolicySummary;
use ubu_core::projection::ProjectionPreview;
use ubu_core::store::{MutationEnvelope, RecalculationTrigger};
use ubu_core::worker::{GpuAdvisoryRequest, GpuAdvisoryResponse};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("UBU_SCHEMAS_FIXTURES"))
}

fn placeholder_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/placeholders")
}

/// Resolve a fixture against the canonical `ubu-schemas` fixtures first, falling
/// back to the `ubu-core`-owned placeholders. Types whose canonical fixtures were
/// removed from `ubu-schemas` (the planning/repair contract types, which are
/// `ubu-core`-owned) resolve against the placeholder tree.
fn resolve_fixture(relative: &str) -> PathBuf {
    let canonical = fixture_root().join(relative);
    if canonical.is_file() {
        canonical
    } else {
        placeholder_root().join(relative)
    }
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
    let path = resolve_fixture(relative);
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
    let path = resolve_fixture(relative);
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
    round_trip_fixture::<Task>("valid/core/task/with-effects.json");
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

#[test]
fn rejects_stale_snapshot_tolerance_fields_fixture() {
    assert_fixture_rejected::<Snapshot>("invalid/core/snapshot/stale-tolerance-fields.json");
}

#[test]
fn rejects_task_effects_unknown_field_fixture() {
    assert_fixture_rejected::<Task>("invalid/core/task/effects-unknown-field.json");
}

#[test]
fn mutation_envelope_fixtures_round_trip_byte_identically() {
    for case in [
        "basic",
        "create-absence-precondition",
        "overnight-advisory",
        "with-policy-versions",
    ] {
        let relative = format!("valid/store/mutation-envelope/{case}.json");
        round_trip_fixture::<MutationEnvelope>(&relative);
        let original = fs::read_to_string(resolve_fixture(&relative)).unwrap();
        let envelope: MutationEnvelope = serde_json::from_str(&original).unwrap();
        envelope.validate().unwrap();
        assert_eq!(
            format!("{}\n", serde_json::to_string_pretty(&envelope).unwrap()),
            original
        );
    }
}

#[test]
fn rejects_malformed_mutation_version_fixture() {
    assert_fixture_rejected::<MutationEnvelope>(
        "invalid/store/mutation-envelope/malformed-version-reference.json",
    );
}

#[test]
fn rejects_leading_zero_mutation_version_fixture() {
    assert_fixture_rejected::<MutationEnvelope>(
        "invalid/store/mutation-envelope/leading-zero-version.json",
    );
}

#[test]
fn device_registration_fixtures_round_trip_byte_identically() {
    for case in ["phase1b-single-device", "revoked"] {
        let relative = format!("valid/core/device-registration/{case}.json");
        round_trip_fixture::<DeviceRegistration>(&relative);
        let original = fs::read_to_string(resolve_fixture(&relative)).unwrap();
        let registration: DeviceRegistration = serde_json::from_str(&original).unwrap();
        registration.validate().unwrap();
        assert_eq!(
            format!("{}\n", serde_json::to_string_pretty(&registration).unwrap()),
            original
        );
    }
}

#[test]
fn rejects_invalid_device_registration_fixtures() {
    for case in ["non-identity-prefix", "missing-required-field", "zone-id-array"] {
        assert_fixture_rejected::<DeviceRegistration>(&format!(
            "invalid/core/device-registration/{case}.json"
        ));
    }
}

#[test]
fn advisory_candidate_fixtures_round_trip_byte_identically() {
    for case in ["proposed-tag", "deferred", "resurfaced", "redacted-payload"] {
        let relative = format!("valid/core/advisory-candidate/{case}.json");
        round_trip_fixture::<AdvisoryCandidate>(&relative);
        let original = fs::read_to_string(resolve_fixture(&relative)).unwrap();
        let candidate: AdvisoryCandidate = serde_json::from_str(&original).unwrap();
        candidate.validate().unwrap();
        assert_eq!(
            format!("{}\n", serde_json::to_string_pretty(&candidate).unwrap()),
            original
        );
    }
}

#[test]
fn suppression_record_fixture_round_trips_byte_identically() {
    let relative = "valid/core/suppression-record/rejected-tag.json";
    round_trip_fixture::<SuppressionRecord>(relative);
    let original = fs::read_to_string(resolve_fixture(relative)).unwrap();
    let record: SuppressionRecord = serde_json::from_str(&original).unwrap();
    record.validate().unwrap();
    assert_eq!(
        format!("{}\n", serde_json::to_string_pretty(&record).unwrap()),
        original
    );
}

#[test]
fn rejects_invalid_advisory_candidate_fixtures() {
    for case in [
        "out-of-range-confidence",
        "missing-required-field",
        "unknown-lifecycle-state",
    ] {
        assert_fixture_rejected::<AdvisoryCandidate>(&format!(
            "invalid/core/advisory-candidate/{case}.json"
        ));
    }
}
