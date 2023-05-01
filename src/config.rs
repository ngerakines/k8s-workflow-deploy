use std::{collections::HashMap, env};

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Stats {
    pub enabled: bool,
    pub statsd_sink: String,
    pub metric_prefix: String,
    pub global_tags: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub stats: Stats,
}

impl Settings {
    pub(crate) fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            .add_source(File::with_name("default"))
            .add_source(File::with_name(&run_mode).required(false))
            .add_source(File::with_name("local").required(false))
            .add_source(Environment::with_prefix("kwd"))
            .build()?;

        s.try_deserialize()
    }
}
