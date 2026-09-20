pub mod authority;
pub mod gpu_advisory;
pub mod local_advisory;
pub mod submission;

pub use authority::WorkerAuthority;
pub use gpu_advisory::{GpuAdvisoryRecommendation, GpuAdvisoryRequest, GpuAdvisoryResponse};
pub use local_advisory::{
    AdvisoryCapability, AdvisoryTransport, ComputeBudget, LocalAdvisoryResult,
    LocalAdvisoryResultStatus, LocalAdvisorySubmission, ProviderConfig,
};
pub use submission::{WorkerResult, WorkerResultStatus, WorkerSubmission};
