use thiserror::Error;

pub type Result<T> = std::result::Result<T, UbuError>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UbuError {
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
}
