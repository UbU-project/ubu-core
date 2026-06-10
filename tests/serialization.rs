use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use ubu_core::core::{ExternalReference, LogEntry, Objective, Task};
use ubu_core::planning::{PlanningRequest, PlanningResponse};
use ubu_core::projection::ProjectionPreview;

fn fixture(relative: &str) -> String {
    let path = PathBuf::from(env!("UBU_SCHEMAS_FIXTURES")).join(relative);
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
    round_trip::<PlanningRequest>("valid/planning/planning-request/basic.json");
    round_trip::<PlanningResponse>("valid/planning/planning-response/basic.json");
    round_trip::<ProjectionPreview>("valid/projection/projection-preview/basic.json");
}
