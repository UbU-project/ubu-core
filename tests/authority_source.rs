use ubu_core::AuthoritySource;

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
