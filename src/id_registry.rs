use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    Task,
    Objective,
    Plan,
    LogEntry,
    ExternalReference,
    Compartment,
    Snapshot,
    AutomationWorker,
    ProjectionPreview,
    Calendar,
}

impl ObjectType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Task => "Task",
            Self::Objective => "Objective",
            Self::Plan => "Plan",
            Self::LogEntry => "LogEntry",
            Self::ExternalReference => "ExternalReference",
            Self::Compartment => "Compartment",
            Self::Snapshot => "Snapshot",
            Self::AutomationWorker => "AutomationWorker",
            Self::ProjectionPreview => "ProjectionPreview",
            Self::Calendar => "Calendar",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefixEntry {
    pub prefix: &'static str,
    pub object_type: ObjectType,
}

pub const PREFIX_REGISTRY: &[PrefixEntry] = &[
    PrefixEntry {
        prefix: "task_",
        object_type: ObjectType::Task,
    },
    PrefixEntry {
        prefix: "obj_",
        object_type: ObjectType::Objective,
    },
    PrefixEntry {
        prefix: "plan_",
        object_type: ObjectType::Plan,
    },
    PrefixEntry {
        prefix: "log_",
        object_type: ObjectType::LogEntry,
    },
    PrefixEntry {
        prefix: "xref_",
        object_type: ObjectType::ExternalReference,
    },
    PrefixEntry {
        prefix: "comp_",
        object_type: ObjectType::Compartment,
    },
    PrefixEntry {
        prefix: "snap_",
        object_type: ObjectType::Snapshot,
    },
    PrefixEntry {
        prefix: "worker_",
        object_type: ObjectType::AutomationWorker,
    },
    PrefixEntry {
        prefix: "proj_",
        object_type: ObjectType::ProjectionPreview,
    },
    PrefixEntry {
        prefix: "cal_",
        object_type: ObjectType::Calendar,
    },
];

pub fn prefix_for(object_type: ObjectType) -> &'static str {
    PREFIX_REGISTRY
        .iter()
        .find(|entry| entry.object_type == object_type)
        .map(|entry| entry.prefix)
        .expect("all object types are registered")
}

pub fn object_type_for_prefix(prefix: &str) -> Option<ObjectType> {
    PREFIX_REGISTRY
        .iter()
        .find(|entry| entry.prefix == prefix)
        .map(|entry| entry.object_type)
}

pub fn prefix_entries() -> &'static [PrefixEntry] {
    PREFIX_REGISTRY
}
