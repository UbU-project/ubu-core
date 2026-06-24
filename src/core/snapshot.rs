use serde::{Deserialize, Serialize};

use crate::object_ref::ObjectRef;
use crate::time::UbuTimestamp;
use crate::UbuId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AffectSourceKind {
    LiveObservation,
    BootstrapDefaultProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AffectDimension {
    Energy,
    Stress,
    MoodIntensity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AffectDimensionObservation {
    pub dimension: AffectDimension,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AffectDimensions {
    pub energy: AffectDimensionObservation,
    pub stress: AffectDimensionObservation,
    pub mood_intensity: AffectDimensionObservation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotAffect {
    pub source_kind: AffectSourceKind,
    pub observed_at: UbuTimestamp,
    pub dimensions: AffectDimensions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: UbuId,
    pub captured_at: UbuTimestamp,
    pub objects: Vec<ObjectRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affect: Option<SnapshotAffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}
