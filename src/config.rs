use std::env;

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub(crate) struct Http {
    pub(crate) enabled: bool,
    pub(crate) port: u16,
    pub(crate) address: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub(crate) struct Https {
    pub(crate) enabled: bool,
    pub(crate) port: u16,
    pub(crate) address: String,
    pub(crate) certificate: String,
    pub(crate) certificate_key: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    pub(crate) http: Http,
    pub(crate) https: Option<Https>,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            .add_source(File::with_name("default"))
            .add_source(File::with_name(&run_mode).required(false))
            .add_source(File::with_name("local").required(false))
            .add_source(Environment::with_prefix("kwd"))
            .build()?;

        info!("http: {:?}", s.get_bool("http.enabled"));
        info!("https: {:?}", s.get_bool("https.enabled"));

        s.try_deserialize()
    }
}
