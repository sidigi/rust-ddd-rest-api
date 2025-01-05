use serde::{Deserialize, Serialize};
use crate::domain::clip::ClipError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content(String);

impl Content {
    pub fn new(content: &str) -> Result<Self,ClipError> {
        if content.is_empty() {
            Err(ClipError::EmptyContent)
        } else {
            Ok(Self(content.to_owned()))
        }
    }

    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}