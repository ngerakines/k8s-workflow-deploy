use anyhow::{anyhow, Result};
use std::{collections::HashMap, env};

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Reconciler {
    pub delay_seconds: u32,
    pub initial_delay_seconds: u32,
}

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
    pub reconciler: Reconciler,
}

impl Settings {
    pub(crate) fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            .add_source(File::with_name("default"))
            .add_source(File::with_name(&run_mode).required(false))
            .add_source(File::with_name("local").required(false))
            .add_source(Environment::with_prefix("kwd_"))
            .build()?;

        s.try_deserialize()
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.stats.enabled && self.stats.statsd_sink.is_empty() {
            return Err(anyhow!("statsd_sink must be set when stats are enabled"));
        }
        if self.reconciler.initial_delay_seconds < 15 {
            return Err(anyhow!(
                "reconciler.initial_delay_seconds must be at least 15 seconds"
            ));
        }
        if self.reconciler.delay_seconds < 60 {
            return Err(anyhow!("reconciler.delay_seconds must at least 60 seconds"));
        }

        Ok(())
    }
}
