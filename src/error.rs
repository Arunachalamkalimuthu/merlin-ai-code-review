//! Unified error type and [`Result`] alias for the entire Merlin codebase.
//!
//! All fallible functions return [`Result<T>`] which resolves to
//! `std::result::Result<T, MerlinError>`.

use thiserror::Error;

/// The top-level error type for Merlin.
///
/// Every public fallible function in this crate returns [`Result<T>`].
///
/// # Automatic conversions
///
/// | Source type | Variant |
/// |-------------|---------|
/// | [`reqwest::Error`] | [`MerlinError::Http`] |
/// | [`serde_json::Error`] | [`MerlinError::Json`] |
/// | [`std::io::Error`] | [`MerlinError::Io`] |
/// | [`toml::de::Error`] | [`MerlinError::TomlDe`] |
#[derive(Debug, Error)]
pub enum MerlinError {
    /// A required configuration field is missing or invalid.
    ///
    /// Raised by [`crate::config::Config`] loaders or platform/AI factory
    /// functions when a mandatory setting is absent.
    #[error("Configuration error: {0}")]
    Config(String),

    /// The unified diff string could not be parsed into [`crate::diff::FileDiff`] structs.
    ///
    /// Raised by [`crate::diff::parse_diff`].
    #[error("Diff parse error: {0}")]
    DiffParse(String),

    /// An AI backend returned an error or an unparsable response.
    ///
    /// Raised by [`crate::ai::AiProvider`] implementations when the upstream
    /// API call fails or returns JSON that cannot be deserialised into
    /// [`crate::ai::ReviewComment`] objects.
    #[error("AI provider error: {0}")]
    AiProvider(String),

    /// A VCS platform API call failed.
    ///
    /// Raised by [`crate::platform::PlatformClient`] implementations.
    #[error("Platform API error: {0}")]
    Platform(String),

    /// An HTTP request failed at the transport layer.
    ///
    /// Automatically converted from [`reqwest::Error`] via `?`.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialisation or deserialisation failed.
    ///
    /// Automatically converted from [`serde_json::Error`] via `?`.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// A filesystem I/O operation failed.
    ///
    /// Automatically converted from [`std::io::Error`] via `?`.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A TOML configuration file could not be deserialised.
    ///
    /// Automatically converted from [`toml::de::Error`] via `?`.
    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    /// A required environment variable is not set.
    ///
    /// The contained string is the name of the missing variable,
    /// e.g. `"ANTHROPIC_API_KEY"`.
    #[error("Environment variable missing: {0}")]
    EnvVar(String),

    /// A catch-all variant for errors that don't fit the categories above.
    #[error("{0}")]
    Other(String),
}

/// Convenience alias for `std::result::Result<T, `[`MerlinError`]`>`.
///
/// Used by every fallible function throughout this crate.
pub type Result<T> = std::result::Result<T, MerlinError>;
