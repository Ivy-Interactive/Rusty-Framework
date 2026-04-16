use serde::{Deserialize, Serialize};

/// Icon identifiers matching common icon libraries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Icon(pub String);

impl Icon {
    pub fn new(name: &str) -> Self {
        Icon(name.to_string())
    }
}

impl From<&str> for Icon {
    fn from(s: &str) -> Self {
        Icon::new(s)
    }
}
