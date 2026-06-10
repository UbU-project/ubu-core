use ubu_core::time::UbuTimestamp;

#[test]
fn accepts_rfc3339_timestamps_with_offsets() {
    UbuTimestamp::parse("2026-06-10T14:30:00Z").expect("UTC timestamp");
    UbuTimestamp::parse("2026-06-10T09:00:00-04:00").expect("offset timestamp");
}

#[test]
fn rejects_naive_timestamps() {
    assert!(UbuTimestamp::parse("2026-06-10T14:30:00").is_err());
}
