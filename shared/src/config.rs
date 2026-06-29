use crate::error::{Result, SeraphError};

/// Runtime configuration loaded from environment variables.
///
/// Call [`Config::from_env`] after `dotenvy::dotenv().ok()` in each binary's
/// main function.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub alchemy_api_key: String,
    pub helius_api_key: String,
    /// Secret used to verify Helius webhook signatures.
    pub helius_webhook_secret: String,
    pub sentry_dsn: Option<String>,
    /// TCP port the API server binds to.  Defaults to 3000.
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: require_env("DATABASE_URL")?,
            redis_url: require_env("REDIS_URL")?,
            alchemy_api_key: require_env("ALCHEMY_API_KEY")?,
            helius_api_key: require_env("HELIUS_API_KEY")?,
            helius_webhook_secret: require_env("HELIUS_WEBHOOK_SECRET")?,
            sentry_dsn: std::env::var("SENTRY_DSN").ok(),
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3000),
        })
    }
}

fn require_env(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| SeraphError::Config(key.to_owned()))
}
