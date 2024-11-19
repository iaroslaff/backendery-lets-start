use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AppConfigs {
    pub(super) allow_cors_origins: Vec<String>,

    pub message_from_email: String,
    pub message_to_email: String,

    pub retry_count: usize,
    pub retry_timeout: u64,

    pub smtp_connection_timeout: u64
}

impl AppConfigs {
    pub(super) fn new(fpath: &str) -> Result<Self, ConfigError> {
        let builder =
            Config::builder().add_source(File::with_name(fpath).required(true));
        let configs: AppConfigs = builder.build()?.try_deserialize()?;

        Ok(configs)
    }
}
