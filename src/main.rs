use anyhow::Result;
use std::borrow::BorrowMut;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::broadcast::Receiver;
#[cfg(debug_assertions)]
use tracing::warn;
use tracing::{error, info};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use kube::CustomResourceExt;

mod config;
mod context;
mod crd;
mod crd_storage;
mod watch_namespace;

use crate::config::Settings;
use crate::crd::Workflow;
use crate::crd_storage::get_workflow_storage;
use crate::watch_namespace::watch_namespace;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        // Enable after https://github.com/tokio-rs/tracing/pull/2566
        // .with(tracing_subscriber::EnvFilter::new(
        //     std::env::var("RUST_LOG")
        //         .unwrap_or_else(|_| "debug".into()),
        // ))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    #[cfg(debug_assertions)]
    warn!("Debug assertions enabled");

    let settings = Settings::new()?;

    println!("crd: {}", serde_yaml::to_string(&Workflow::crd()).unwrap());

    let workflow_storage = get_workflow_storage("memory");

    let app_context = context::Context(Arc::new(context::InnerContext::new(
        settings.clone(),
        workflow_storage,
    )));

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<bool>(100);
    let (rev_shutdown_tx, mut rev_shutdown_rx) = tokio::sync::broadcast::channel::<bool>(100);

    let loop_join_handler = tokio::spawn(async move {
        let loop_rx = shutdown_rx.borrow_mut();
        if let Err(err) = watch_namespace(settings, app_context, loop_rx).await {
            error!(cause = ?err, "metric_loop error");
            rev_shutdown_tx.send(true).unwrap();
        }
    });

    shutdown_signal(rev_shutdown_rx.borrow_mut()).await;

    shutdown_tx.send(true)?;

    loop_join_handler.await?;

    Ok(())
}

async fn shutdown_signal(rx: &mut Receiver<bool>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
        _ = rx.recv() => {},
    }

    info!("signal received, starting graceful shutdown");
}
