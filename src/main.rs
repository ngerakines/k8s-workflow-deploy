use anyhow::Result;

#[cfg(debug_assertions)]
use tracing::warn;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;

use crate::config::Settings;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        // Enable after https://github.com/tokio-rs/tracing/pull/2566
        // .with(tracing_subscriber::EnvFilter::new(
        //     std::env::var("RUST_LOG")
        //         .unwrap_or_else(|_| "debug".into()),
        // ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    #[cfg(debug_assertions)]
    warn!("Debug assertions enabled");

    let _ = Settings::new()?;

    Ok(())
}
