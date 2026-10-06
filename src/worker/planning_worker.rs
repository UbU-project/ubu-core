//! Canonical response envelopes and engine provenance; typed planning payloads
//! are composed by the kernel, which depends on core rather than vice versa.
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    CpuReference,
    GpuWorker,
    MobileCpu,
    MobileGpu,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationKind {
    InProcessCpu,
    PersistentPythonWorker,
    InProcessMobile,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuCertificationStatus {
    NotYetCertified,
    Certified,
    RejectedByCpu,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineProvenance {
    pub backend_kind: BackendKind,
    pub invocation_kind: InvocationKind,
    pub engine_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framework_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_profile: Option<String>,
    pub cpu_certification_status: CpuCertificationStatus,
}
impl EngineProvenance {
    pub fn cpu(version: &str) -> Self {
        Self {
            backend_kind: BackendKind::CpuReference,
            invocation_kind: InvocationKind::InProcessCpu,
            engine_version: version.into(),
            framework: None,
            framework_version: None,
            device_summary: Some("cpu".into()),
            tolerance_profile: None,
            cpu_certification_status: CpuCertificationStatus::Certified,
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.engine_version.is_empty()
            || [
                &self.framework,
                &self.framework_version,
                &self.device_summary,
                &self.tolerance_profile,
            ]
            .into_iter()
            .any(|s| s.as_ref().is_some_and(String::is_empty))
        {
            return Err("provenance strings must be nonempty");
        }
        if self.backend_kind == BackendKind::GpuWorker
            && self.framework.as_deref() != Some("pytorch")
        {
            return Err("gpu_worker requires framework pytorch");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameType {
    ChunkResult,
    FinalResponse,
    EngineError,
    Cancelled,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningStreamFrame {
    pub schema_version: String,
    pub request_id: String,
    pub frame_index: u64,
    pub frame_type: FrameType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_depth: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partial_response: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
impl PlanningStreamFrame {
    pub fn outcome(request_id: &str, frame_type: FrameType) -> Self {
        Self {
            schema_version: crate::planning::PLANNING_KERNEL_CONTRACT_VERSION.into(),
            request_id: request_id.into(),
            frame_index: 0,
            frame_type,
            chunk_depth: None,
            chunk_id: None,
            partial_response: None,
            response: None,
            error: None,
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != crate::planning::PLANNING_KERNEL_CONTRACT_VERSION
            || self.request_id.is_empty()
        {
            return Err("invalid frame version or request id");
        }
        let chunk = self.chunk_depth.is_some()
            || self.chunk_id.is_some()
            || self.partial_response.is_some();
        let valid = match self.frame_type {
            FrameType::ChunkResult => {
                self.chunk_depth.is_some_and(|d| d > 0)
                    && self.chunk_id.as_ref().is_some_and(|s| !s.is_empty())
                    && self.partial_response.as_ref().is_some_and(Value::is_object)
                    && self.response.is_none()
                    && self.error.is_none()
            }
            FrameType::FinalResponse => {
                !chunk
                    && self.response.as_ref().is_some_and(Value::is_object)
                    && self.error.is_none()
            }
            FrameType::EngineError => {
                !chunk
                    && self.response.is_none()
                    && self.error.as_ref().is_some_and(|s| !s.is_empty())
            }
            FrameType::Cancelled => !chunk && self.response.is_none() && self.error.is_none(),
        };
        if valid {
            Ok(())
        } else {
            Err("frame payload does not match frame_type")
        }
    }
}
pub fn validate_sequence(frames: &[PlanningStreamFrame]) -> Result<(), &'static str> {
    let Some(first) = frames.first() else {
        return Err("empty frame sequence");
    };
    if first.frame_index != 0 {
        return Err("first frame_index must be zero");
    }
    for (i, frame) in frames.iter().enumerate() {
        frame.validate()?;
        if frame.request_id != first.request_id
            || (i > 0 && frames[i - 1].frame_index >= frame.frame_index)
        {
            return Err("frame identity or ordering violation");
        }
        let terminal = frame.frame_type != FrameType::ChunkResult;
        if terminal != (i == frames.len() - 1) {
            return Err("exactly one terminal frame must end the sequence");
        }
    }
    Ok(())
}
