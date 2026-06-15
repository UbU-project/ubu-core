use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

use regex::Regex;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::errors::UbuError;
use crate::id_registry::{object_type_for_prefix, prefix_for, ObjectType};

static ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-z]+_[0-9a-f]{12}7[0-9a-f]{3}[89ab][0-9a-f]{15}$").expect("valid UbU id regex")
});

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UbuId(String);

impl UbuId {
    pub fn parse(value: impl Into<String>) -> crate::Result<Self> {
        let value = value.into();
        if !ID_RE.is_match(&value) {
            return Err(UbuError::InvalidId { value });
        }
        let prefix = id_prefix(&value).expect("validated id has prefix delimiter");
        if object_type_for_prefix(prefix).is_none() {
            return Err(UbuError::UnknownIdPrefix {
                prefix: prefix.to_owned(),
            });
        }
        Ok(Self(value))
    }

    pub fn new(object_type: ObjectType) -> Self {
        let suffix = Uuid::now_v7().simple().to_string();
        Self(format!("{}{}", prefix_for(object_type), suffix))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn prefix(&self) -> &'static str {
        prefix_for(self.object_type())
    }

    pub fn object_type(&self) -> ObjectType {
        object_type_for_prefix(id_prefix(&self.0).expect("validated id has prefix delimiter"))
            .expect("validated id has registered prefix")
    }

    pub fn require_object_type(&self, expected: ObjectType) -> crate::Result<()> {
        let actual = self.object_type();
        if actual == expected {
            return Ok(());
        }

        Err(UbuError::WrongIdObjectType {
            id: self.0.clone(),
            expected: expected.as_str(),
            actual: actual.as_str(),
        })
    }
}

fn id_prefix(value: &str) -> Option<&str> {
    let delimiter = value.find('_')?;
    Some(&value[..=delimiter])
}

impl fmt::Display for UbuId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for UbuId {
    type Err = UbuError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Serialize for UbuId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for UbuId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(D::Error::custom)
    }
}
