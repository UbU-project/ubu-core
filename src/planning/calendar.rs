use serde::{Deserialize, Serialize};

use crate::time::UbuTimestamp;
use crate::UbuId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarWindow {
    pub start: UbuTimestamp,
    pub end: UbuTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub id: UbuId,
    pub timezone: String,
    pub windows: Vec<CalendarWindow>,
}
