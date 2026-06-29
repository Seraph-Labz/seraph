use thiserror::Error;

#[derive(Debug, Error)]
pub enum SeraphError {
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),

    #[error("migration: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("parse failed: {0}")]
    Parse(String),

    #[error("config: missing env var {0}")]
    Config(String),

    #[error("{0} not found")]
    NotFound(String),

    #[error("protocol {protocol}: {message}")]
    Protocol { protocol: String, message: String },
}

pub type Result<T> = std::result::Result<T, SeraphError>;
