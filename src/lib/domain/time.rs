use std::str::FromStr;
use chrono::{DateTime, NaiveDateTime, Utc};
use derive_more::From;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub struct Time(DateTime<Utc>);

impl Time {
    pub fn timestamp(&self) -> i64 {
        self.0.timestamp()
    }

    pub fn into_inner(self) -> DateTime<Utc> {
        self.0
    }

    pub fn from_naive_utc(date_time: NaiveDateTime) -> Self {
        Self(DateTime::from_utc(date_time, Utc))
    }
}

impl FromStr for Time {
    type Err = chrono::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match format!("{}T00:00:00Z", s).parse::<DateTime<Utc>>() {
            Ok(time) => Ok(time.into()),
            Err(e) => Err(e),
        }
    }

}