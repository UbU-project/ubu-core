use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use ubu_core::{AuthoritySource, Provenance};

#[test]
fn authority_source_round_trips_all_members() {
    let cases = [
        ("user", AuthoritySource::User),
        ("user_override", AuthoritySource::UserOverride),
        ("delegated", AuthoritySource::Delegated),
        ("automation_worker", AuthoritySource::AutomationWorker),
        ("policy", AuthoritySource::Policy),
        ("system", AuthoritySource::System),
    ];

    for (wire, expected) in cases {
        let parsed: AuthoritySource = serde_json::from_str(&format!("\"{wire}\"")).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(
            serde_json::to_value(parsed).unwrap(),
            serde_json::json!(wire)
        );
    }
}

#[test]
fn unknown_authority_source_fails() {
    assert!(serde_json::from_str::<AuthoritySource>("\"operator\"").is_err());
}

#[test]
fn deprecated_authority_source_values_fail() {
    for wire in [
        "github_event",
        "imported_config",
        "model_generated",
        "human_admin",
        "project_policy",
    ] {
        assert!(
            serde_json::from_str::<AuthoritySource>(&format!("\"{wire}\"")).is_err(),
            "{wire} should not deserialize as AuthoritySource"
        );
    }
}

#[test]
fn provenance_round_trips_source_refs_fixture() {
    let fixture = fixture("valid/common/provenance/system-github-source-ref.json");
    let original: Value = serde_json::from_str(&fixture).expect("valid fixture json");
    let parsed: Provenance = serde_json::from_value(original.clone()).expect("deserialize");

    assert_eq!(parsed.authority_source, AuthoritySource::System);
    assert!(parsed
        .source_refs
        .as_ref()
        .is_some_and(|refs| refs.len() == 1));
    assert_eq!(serde_json::to_value(parsed).expect("serialize"), original);
}

#[test]
fn deprecated_authority_source_fails_in_provenance_fixture() {
    let fixture = fixture("invalid/common/provenance/github-event-authority.json");

    assert!(serde_json::from_str::<Provenance>(&fixture).is_err());
}

fn fixture(relative: &str) -> String {
    let path = PathBuf::from(env!("UBU_SCHEMAS_FIXTURES")).join(relative);
    fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("failed to read fixture {}: {err}", path.display());
    })
}
