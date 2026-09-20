use thiserror::Error;

pub type Result<T> = std::result::Result<T, UbuError>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UbuError {
    #[error("zone id must not be empty")]
    EmptyZoneId,

    #[error("device label must not be empty")]
    EmptyDeviceLabel,

    #[error("device capability must not be empty")]
    EmptyDeviceCapability,

    #[error("origin device id must not be empty")]
    EmptyDeviceId,

    #[error("idempotency key must not be empty")]
    EmptyIdempotencyKey,

    #[error("invalid version reference `{value}`")]
    InvalidVersionRef { value: String },

    #[error("policy-dependent mutation requires non-empty observed_policy_versions")]
    MissingObservedPolicyVersions,

    #[error("idempotency_key_conflict for device `{origin_device_id}` and key `{idempotency_key}`")]
    IdempotencyKeyConflict {
        origin_device_id: String,
        idempotency_key: String,
    },

    #[error("invalid UbU id `{value}`")]
    InvalidId { value: String },

    #[error("unknown UbU id prefix `{prefix}`")]
    UnknownIdPrefix { prefix: String },

    #[error("id `{id}` has object type `{actual}`, expected `{expected}`")]
    WrongIdObjectType {
        id: String,
        expected: &'static str,
        actual: &'static str,
    },

    #[error("invalid RFC 3339 timestamp with required timezone offset `{value}`")]
    InvalidTimestamp { value: String },

    #[error("invalid compartment label `{value}`")]
    InvalidCompartmentLabel { value: String },

    #[error("task status `moot` requires moot_reason_code")]
    MissingMootReasonCode,

    #[error("task status `{status}` forbids moot_reason_code")]
    UnexpectedMootReasonCode { status: &'static str },
}
