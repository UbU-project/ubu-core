use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{ids::UbuId, provenance::Provenance, time::UbuTimestamp, ObjectType, UbuError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreferenceSubjects {
    Tasks { a: UbuId, b: UbuId },
    Objectives { a: UbuId, b: UbuId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceOrder {
    APreferredToB,
    AIndifferentToB,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceAcquiredMethod {
    UserDefined,
    LlmEstimated,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Preference {
    pub id: UbuId,
    pub subjects: PreferenceSubjects,
    pub order: PreferenceOrder,
    pub acquired_method: PreferenceAcquiredMethod,
    pub acquired_date: UbuTimestamp,
    pub enabled: bool,
    pub provenance: Provenance,
}

// Absent subjects are permitted until pair validation, but explicit null is not.
fn subject<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<UbuId>, D::Error> {
    UbuId::deserialize(deserializer)
        .map(Some)
        .map_err(|_| D::Error::custom(UbuError::InvalidPreferenceSubjects))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreferenceWire {
    id: UbuId,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "subject"
    )]
    task_a: Option<UbuId>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "subject"
    )]
    task_b: Option<UbuId>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "subject"
    )]
    objective_a: Option<UbuId>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "subject"
    )]
    objective_b: Option<UbuId>,
    order: PreferenceOrder,
    acquired_method: PreferenceAcquiredMethod,
    acquired_date: UbuTimestamp,
    enabled: bool,
    provenance: Provenance,
}

impl Preference {
    pub fn validate(&self) -> crate::Result<()> {
        self.id.require_object_type(ObjectType::Preference)?;
        let (a, b, kind) = match &self.subjects {
            PreferenceSubjects::Tasks { a, b } => (a, b, ObjectType::Task),
            PreferenceSubjects::Objectives { a, b } => (a, b, ObjectType::Objective),
        };
        for id in [a, b] {
            id.require_object_type(kind)
                .map_err(|_| UbuError::InvalidPreferenceSubjects)?;
        }
        if a == b {
            return Err(UbuError::PreferenceSelfRelation);
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for Preference {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = PreferenceWire::deserialize(deserializer)?;
        let subjects = match (wire.task_a, wire.task_b, wire.objective_a, wire.objective_b) {
            (Some(a), Some(b), None, None) => PreferenceSubjects::Tasks { a, b },
            (None, None, Some(a), Some(b)) => PreferenceSubjects::Objectives { a, b },
            _ => return Err(D::Error::custom(UbuError::InvalidPreferenceSubjects)),
        };
        let preference = Self {
            id: wire.id,
            subjects,
            order: wire.order,
            acquired_method: wire.acquired_method,
            acquired_date: wire.acquired_date,
            enabled: wire.enabled,
            provenance: wire.provenance,
        };
        preference.validate().map_err(D::Error::custom)?;
        Ok(preference)
    }
}

impl Serialize for Preference {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (task_a, task_b, objective_a, objective_b) = match &self.subjects {
            PreferenceSubjects::Tasks { a, b } => (Some(a.clone()), Some(b.clone()), None, None),
            PreferenceSubjects::Objectives { a, b } => {
                (None, None, Some(a.clone()), Some(b.clone()))
            }
        };
        PreferenceWire {
            id: self.id.clone(),
            task_a,
            task_b,
            objective_a,
            objective_b,
            order: self.order,
            acquired_method: self.acquired_method,
            acquired_date: self.acquired_date.clone(),
            enabled: self.enabled,
            provenance: self.provenance.clone(),
        }
        .serialize(serializer)
    }
}
