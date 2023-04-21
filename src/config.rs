use std::env;

use derive_builder::Builder;

#[derive(Builder, Clone, Debug)]
#[builder(setter(into, strip_option))]
pub struct Config {
    #[builder(setter(into), default = "self.default_version()")]
    pub version: String,

    #[builder(setter(into), default = "self.default_port()")]
    pub port: u16,

    #[builder(setter(into), default = "self.default_secure_port()")]
    pub secure_port: u16,

    #[builder(setter(into), default = "self.default_certificate()")]
    pub certificate: String,

    #[builder(setter(into), default = "self.default_certificate_key()")]
    pub certificate_key: String,
}

impl ConfigBuilder {
    fn default_version(&self) -> String {
        option_env!("GIT_HASH")
            .unwrap_or(env!("CARGO_PKG_VERSION", "develop"))
            .to_string()
    }

    fn default_port(&self) -> u16 {
        env::var("PORT")
            .unwrap_or("8080".to_string())
            .parse::<u16>()
            .unwrap_or(8080)
    }

    fn default_secure_port(&self) -> u16 {
        env::var("SECURE_PORT")
            .unwrap_or("8443".to_string())
            .parse::<u16>()
            .unwrap_or(8443)
    }

    fn default_certificate(&self) -> String {
        env::var("CERTIFICATE").unwrap_or("".to_string())
    }

    fn default_certificate_key(&self) -> String {
        env::var("CERTIFICATE_KEY").unwrap_or("".to_string())
    }
}
