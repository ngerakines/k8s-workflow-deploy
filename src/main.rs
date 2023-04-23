use anyhow::Result;
use std::borrow::BorrowMut;
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::broadcast::Receiver;

#[cfg(debug_assertions)]
use tracing::warn;

use tracing::{error, info};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use kube::CustomResourceExt;

mod action;
mod action_loop;
mod config;
mod context;
mod crd;
mod crd_storage;
mod k8s_util;
mod reconcile;
mod watch_deployment;
mod watch_namespace;
mod watch_workflow;

use crate::action::Action;
use crate::action_loop::action_loop;
use crate::config::Settings;
use crate::crd::Workflow;
use crate::crd_storage::get_workflow_storage;
use crate::reconcile::reconcile_loop;
use crate::watch_deployment::watch_deployment;
use crate::watch_namespace::watch_namespace;
use crate::watch_workflow::{init_workflow_crd, watch_workflow};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "k8s_workflow_deploy=debug,info".into()),
        ))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    #[cfg(debug_assertions)]
    warn!("Debug assertions enabled");

    let settings = Settings::new()?;

    println!("crd: {}", serde_yaml::to_string(&Workflow::crd()).unwrap());

    let workflow_storage = get_workflow_storage("memory");

    let (action_tx, mut action_rx) = tokio::sync::mpsc::channel::<Action>(100);

    let app_context = context::Context(Arc::new(context::InnerContext::new(
        settings.clone(),
        workflow_storage,
        action_tx.clone(),
    )));

    init_workflow_crd().await?;

    let (shutdown_tx, _) = tokio::sync::broadcast::channel::<bool>(100);
    let (rev_shutdown_tx, mut rev_shutdown_rx) = tokio::sync::broadcast::channel::<bool>(100);

    let namespace_join_handler = {
        let d_shutdown_tx = shutdown_tx.clone();
        let settings = settings.clone();
        let app_context = app_context.clone();
        let ns_rev_shutdown_tx = rev_shutdown_tx.clone();
        tokio::spawn(async move {
            let mut loop_rx = d_shutdown_tx.subscribe();
            if let Err(err) = watch_namespace(settings, app_context, &mut loop_rx).await {
                error!(cause = ?err, "watch_namespace error");
                ns_rev_shutdown_tx.send(true).unwrap();
            }
        })
    };

    let deployment_join_handler = {
        let d_shutdown_tx = shutdown_tx.clone();
        let settings = settings.clone();
        let app_context = app_context.clone();
        let d_rev_shutdown_tx = rev_shutdown_tx.clone();
        tokio::spawn(async move {
            let mut loop_rx = d_shutdown_tx.subscribe();
            if let Err(err) = watch_deployment(settings.clone(), app_context, &mut loop_rx).await {
                error!(cause = ?err, "watch_deployment error");
                d_rev_shutdown_tx.send(true).unwrap();
            }
        })
    };

    let workflow_join_handler = {
        let w_shutdown_tx = shutdown_tx.clone();
        let settings = settings.clone();
        let app_context = app_context.clone();
        let w_rev_shutdown_tx = rev_shutdown_tx.clone();
        tokio::spawn(async move {
            let mut loop_rx = w_shutdown_tx.subscribe();
            if let Err(err) = watch_workflow(settings.clone(), app_context, &mut loop_rx).await {
                error!(cause = ?err, "watch_workflow error");
                w_rev_shutdown_tx.send(true).unwrap();
            }
        })
    };

    let reconcile_join_handler = {
        let r_shutdown_tx = shutdown_tx.clone();
        let settings = settings.clone();
        let app_context = app_context.clone();
        let r_rev_shutdown_tx = rev_shutdown_tx.clone();
        tokio::spawn(async move {
            let mut loop_rx = r_shutdown_tx.subscribe();
            if let Err(err) = reconcile_loop(settings.clone(), app_context, &mut loop_rx).await {
                error!(cause = ?err, "reconcile_loop error");
                r_rev_shutdown_tx.send(true).unwrap();
            }
        })
    };

    let action_join_handler = {
        let a_shutdown_tx = shutdown_tx.clone();
        let settings = settings.clone();
        let app_context = app_context.clone();
        let a_rev_shutdown_tx = rev_shutdown_tx.clone();
        tokio::spawn(async move {
            let mut loop_rx = a_shutdown_tx.subscribe();
            if let Err(err) =
                action_loop(settings.clone(), app_context, &mut loop_rx, &mut action_rx).await
            {
                error!(cause = ?err, "action_loop error");
                a_rev_shutdown_tx.send(true).unwrap();
            }
        })
    };

    OpenOptions::new()
        .create(true)
        .write(true)
        .open(Path::new("/tmp/started"))?;
    OpenOptions::new()
        .create(true)
        .write(true)
        .open(Path::new("/tmp/alive"))?;
    OpenOptions::new()
        .create(true)
        .write(true)
        .open(Path::new("/tmp/ready"))?;

    shutdown_signal(rev_shutdown_rx.borrow_mut()).await;

    shutdown_tx.send(true)?;

    namespace_join_handler.await?;
    deployment_join_handler.await?;
    workflow_join_handler.await?;
    reconcile_join_handler.await?;
    action_join_handler.await?;

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
