use anyhow::Result;
use futures::prelude::*;
use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    api::{Api, ListParams},
    runtime::watcher,
    Client,
};
use tokio::sync::broadcast::Receiver;
use tracing::{error, info};

use crate::{config::Settings, context::Context};

pub(crate) async fn watch_deployment(
    _settings: Settings,
    _context: Context,
    shutdown: &mut Receiver<bool>,
) -> Result<()> {
    let client = Client::try_default().await.map_err(anyhow::Error::msg)?;
    let api = Api::<Deployment>::all(client.clone());

    info!("kubernetes deployment watcher started");

    let deployment_watcher = watcher(api, ListParams::default()).try_for_each(|event| async {
        match event {
            kube::runtime::watcher::Event::Deleted(_deployment) => {
                // TODO: Don't unwatch deployments that aren't annotated.
            }
            kube::runtime::watcher::Event::Applied(_deployment) => {
                // TODO: Look for annotation removal and unwatch accordingly.
            }
            _ => {}
        }
        Ok(())
    });

    tokio::select! {
        res = deployment_watcher => {
            if let Err(e) = res {
                error!("kubernetes deployment watcher error: {}", e);
            }
        },
        _ = shutdown.recv() => { },
    };

    info!("kubernetes deployment watcher stopped");

    Ok(())
}
