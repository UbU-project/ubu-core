use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::{ObjectType, Provenance, UbuError, UbuId};

use super::{TaskStatus, WorkItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerStatus {
    Active,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerMutationReason {
    Decomposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerCompletion {
    Complete,
    InProgress,
}

/// Admitted structure, never a scheduled Task. Lineage identifies the exact
/// origin version and the mutation Log; completion is derived from children.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ContainerWire")]
pub struct Container {
    pub id: UbuId,
    pub name: String,
    pub status: ContainerStatus,
    pub origin_task_ref: UbuId,
    pub origin_task_version: u64,
    pub mutation_reason: ContainerMutationReason,
    pub mutation_log_ref: UbuId,
    pub items: Vec<WorkItem>,
    pub segment_split_points: Vec<usize>,
    pub provenance: Provenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by_task_ref: Option<UbuId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContainerWire {
    id: UbuId,
    name: String,
    status: ContainerStatus,
    origin_task_ref: UbuId,
    origin_task_version: u64,
    mutation_reason: ContainerMutationReason,
    mutation_log_ref: UbuId,
    items: Vec<WorkItem>,
    segment_split_points: Vec<usize>,
    provenance: Provenance,
    #[serde(default)]
    superseded_by_task_ref: Option<UbuId>,
}

impl TryFrom<ContainerWire> for Container {
    type Error = UbuError;

    fn try_from(wire: ContainerWire) -> crate::Result<Self> {
        let container = Self {
            id: wire.id,
            name: wire.name,
            status: wire.status,
            origin_task_ref: wire.origin_task_ref,
            origin_task_version: wire.origin_task_version,
            mutation_reason: wire.mutation_reason,
            mutation_log_ref: wire.mutation_log_ref,
            items: wire.items,
            segment_split_points: wire.segment_split_points,
            provenance: wire.provenance,
            superseded_by_task_ref: wire.superseded_by_task_ref,
        };
        container.validate()?;
        Ok(container)
    }
}

impl Container {
    pub fn validate(&self) -> crate::Result<()> {
        let invalid = |field| UbuError::InvalidContainer { field };
        self.id.require_object_type(ObjectType::Container)?;
        if self.name.is_empty() {
            return Err(invalid("name"));
        }
        self.origin_task_ref.require_object_type(ObjectType::Task)?;
        self.mutation_log_ref
            .require_object_type(ObjectType::LogEntry)?;
        if self.origin_task_version == 0 {
            return Err(invalid("origin_task_version"));
        }
        if self.items.len() < 2 {
            return Err(invalid("items"));
        }
        for item in &self.items {
            item.object_ref.id.require_object_type(ObjectType::Task)?;
            if item.object_ref.object_type != ObjectType::Task || item.summary.is_empty() {
                return Err(invalid("items"));
            }
        }
        if self
            .segment_split_points
            .iter()
            .any(|&point| point == 0 || point >= self.items.len())
            || self
                .segment_split_points
                .windows(2)
                .any(|points| points[0] >= points[1])
        {
            return Err(invalid("segment_split_points"));
        }
        if self.superseded_by_task_ref.is_some() != (self.status == ContainerStatus::Superseded) {
            return Err(invalid("superseded_by_task_ref"));
        }
        if let Some(id) = &self.superseded_by_task_ref {
            id.require_object_type(ObjectType::Task)?;
        }
        Ok(())
    }

    /// Exclusive boundaries: [2] over four children yields 0..2 and 2..4.
    /// Validate after changing public fields and before deriving these ranges.
    pub fn segments(&self) -> Vec<Range<usize>> {
        let mut start = 0;
        self.segment_split_points
            .iter()
            .copied()
            .chain(std::iter::once(self.items.len()))
            .map(|end| {
                let range = start..end;
                start = end;
                range
            })
            .collect()
    }

    pub fn completion_state(
        &self,
        child_states: &[TaskStatus],
    ) -> crate::Result<ContainerCompletion> {
        if child_states.len() != self.items.len() {
            return Err(UbuError::ContainerChildStateCount {
                expected: self.items.len(),
                actual: child_states.len(),
            });
        }
        Ok(
            if child_states
                .iter()
                .all(|state| matches!(state, TaskStatus::Completed | TaskStatus::Moot))
            {
                ContainerCompletion::Complete
            } else {
                ContainerCompletion::InProgress
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::path::PathBuf;

    fn fixture(kind: &str, name: &str) -> String {
        let relative = format!("{kind}/core/container/{name}.json");
        let canonical = PathBuf::from(env!("UBU_SCHEMAS_FIXTURES")).join(&relative);
        let path = if canonical.is_file() {
            canonical
        } else {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/placeholders")
                .join(relative)
        };
        std::fs::read_to_string(path).unwrap()
    }
    fn container() -> Container {
        serde_json::from_str(&fixture("valid", "two-segments")).unwrap()
    }

    #[test]
    fn valid_shapes_round_trip_byte_identically() {
        for name in ["no-splits", "two-segments", "superseded"] {
            let original = fixture("valid", name);
            let c: Container = serde_json::from_str(&original).unwrap();
            c.validate().unwrap();
            assert_eq!(
                format!("{}\n", serde_json::to_string_pretty(&c).unwrap()),
                original
            );
        }
    }

    #[test]
    fn rejects_schema_cases_and_core_only_bounds_and_references() {
        for name in [
            "zero-split",
            "non-increasing",
            "duplicate-split",
            "single-item",
            "unknown-field",
            "unknown-reason",
        ] {
            assert!(
                serde_json::from_str::<Container>(&fixture("invalid", name)).is_err(),
                "{name}"
            );
        }
        let original: Value = serde_json::from_str(&fixture("valid", "two-segments")).unwrap();
        for (field, value) in [
            ("segment_split_points", json!([4])),
            ("origin_task_version", json!(0)),
            ("origin_task_ref", original["id"].clone()),
            ("mutation_log_ref", original["origin_task_ref"].clone()),
            (
                "superseded_by_task_ref",
                original["origin_task_ref"].clone(),
            ),
            ("status", json!("superseded")),
        ] {
            let mut payload = original.clone();
            payload[field] = value;
            assert!(
                serde_json::from_value::<Container>(payload).is_err(),
                "{field}"
            );
        }
        let mut c = container();
        c.items[0].object_ref.object_type = ObjectType::Objective;
        assert!(c.validate().is_err());
        let mut c = container();
        c.items[0].object_ref.id = c.id.clone();
        assert!(c.validate().is_err());
        let mut c = container();
        c.status = ContainerStatus::Superseded;
        c.superseded_by_task_ref = Some(c.id.clone());
        assert!(c.validate().is_err());
    }

    #[test]
    fn segments_cover_none_one_and_two_split_points() {
        let mut c = container();
        c.segment_split_points.clear();
        assert_eq!(c.segments(), vec![0..4]);
        c.segment_split_points = vec![2];
        assert_eq!(c.segments(), vec![0..2, 2..4]);
        c.segment_split_points = vec![1, 3];
        assert_eq!(c.segments(), vec![0..1, 1..3, 3..4]);
    }

    #[test]
    fn completion_is_derived_and_requires_every_child_state() {
        use TaskStatus::*;
        let c = container();
        assert_eq!(
            c.completion_state(&[Completed; 4]).unwrap(),
            ContainerCompletion::Complete
        );
        assert_eq!(
            c.completion_state(&[Completed, Moot, Completed, Moot])
                .unwrap(),
            ContainerCompletion::Complete
        );
        assert_eq!(
            c.completion_state(&[Completed, Moot, Active, Moot])
                .unwrap(),
            ContainerCompletion::InProgress
        );
        assert_eq!(
            c.completion_state(&[Completed, Moot, Failed, Moot])
                .unwrap(),
            ContainerCompletion::InProgress
        );
        assert!(matches!(
            c.completion_state(&[Completed; 3]),
            Err(UbuError::ContainerChildStateCount {
                expected: 4,
                actual: 3
            })
        ));
    }
}
