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
    Preference,
    Container,
    UniverseState,
    Identity,
    Relationship,
    ExternalEvent,
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
            Self::Preference => "Preference",
            Self::Container => "Container",
            Self::UniverseState => "UniverseState",
            Self::Identity => "Identity",
            Self::Relationship => "Relationship",
            Self::ExternalEvent => "ExternalEvent",
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
    PrefixEntry {
        prefix: "pref_",
        object_type: ObjectType::Preference,
    },
    PrefixEntry {
        prefix: "container_",
        object_type: ObjectType::Container,
    },
    PrefixEntry {
        prefix: "ustate_",
        object_type: ObjectType::UniverseState,
    },
    PrefixEntry {
        prefix: "identity_",
        object_type: ObjectType::Identity,
    },
    PrefixEntry {
        prefix: "rel_",
        object_type: ObjectType::Relationship,
    },
    PrefixEntry {
        prefix: "xevent_",
        object_type: ObjectType::ExternalEvent,
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

pub fn object_type_from_id(value: &str) -> Option<ObjectType> {
    let delimiter = value.find('_')?;
    object_type_for_prefix(&value[..=delimiter])
}

pub fn prefix_entries() -> &'static [PrefixEntry] {
    PREFIX_REGISTRY
}
