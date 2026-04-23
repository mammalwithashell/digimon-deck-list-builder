//! Error types for the DSL.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DslError {
    #[error("IO error loading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("YAML parse error in {path}: {source}")]
    Yaml {
        path: String,
        #[source]
        source: serde_yml::Error,
    },

    #[error("validation failed with {} errors", .0.len())]
    Validation(Vec<ValidationError>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub card_id: String,
    pub path: String, // e.g. "effects[2].process[1].select_hand.filter"
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.card_id, self.path, self.message)
    }
}
