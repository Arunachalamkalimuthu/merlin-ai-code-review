use thiserror::Error;

#[derive(Debug, Error)]
pub enum MerlinError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Diff parse error: {0}")]
    DiffParse(String),

    #[error("AI provider error: {0}")]
    AiProvider(String),

    #[error("Platform API error: {0}")]
    Platform(String),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("Environment variable missing: {0}")]
    EnvVar(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, MerlinError>;
