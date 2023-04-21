use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;

use crate::config::ConfigBuilder;

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "k8s_workflow_deploy=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    #[cfg(debug_assertions)]
    warn!("Debug assertions enabled");

    let settings_builder = ConfigBuilder::default();
    let settings = settings_builder.build().unwrap();

    info!("Starting k8s_workflow_deploy on {}", settings.port);
}
