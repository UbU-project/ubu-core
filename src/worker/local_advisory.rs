//! By-value local advisory boundary (UBU-D0255), with no execution or transport.

use serde::{Deserialize, Serialize};

use crate::CandidateKind;

/// The complete grant vocabulary. There is no write, admission, StateStore,
/// context-fetch, authority-expansion, export-policy, network, tool, credential,
/// or retention capability. Downstream code cannot add enum variants.
/// `#[non_exhaustive]` permits future library evolution, not caller-defined grants.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryCapability {
    ProposeCandidate(CandidateKind),
    EmitDiagnostics,
    EmitTelemetry,
}
