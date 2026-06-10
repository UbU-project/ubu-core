use std::fmt;
use std::str::FromStr;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::errors::UbuError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UbuTimestamp(OffsetDateTime);

impl UbuTimestamp {
    pub fn parse(value: impl AsRef<str>) -> crate::Result<Self> {
        let value = value.as_ref();
        let parsed =
            OffsetDateTime::parse(value, &Rfc3339).map_err(|_| UbuError::InvalidTimestamp {
                value: value.to_owned(),
            })?;
        Ok(Self(parsed))
    }

    pub fn now_utc() -> Self {
        Self(OffsetDateTime::now_utc())
    }

    pub fn inner(self) -> OffsetDateTime {
        self.0
    }
}

impl fmt::Display for UbuTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let formatted = self.0.format(&Rfc3339).map_err(|_| fmt::Error)?;
        f.write_str(&formatted)
    }
}

impl FromStr for UbuTimestamp {
    type Err = UbuError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Serialize for UbuTimestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for UbuTimestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(D::Error::custom)
    }
}
