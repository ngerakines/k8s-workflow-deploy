use anyhow::Result;
use futures::prelude::*;
use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::{Api, ListParams},
    runtime::watcher,
    Client,
};
use tokio::sync::broadcast::Receiver;
use tracing::{error, info};

use crate::{config::Settings, context::Context};

pub(crate) async fn watch_namespace(
    _settings: Settings,
    _context: Context,
    shutdown: &mut Receiver<bool>,
) -> Result<()> {
    let client = Client::try_default().await.map_err(anyhow::Error::msg)?;
    let api = Api::<Namespace>::all(client.clone());

    info!("kubernetes namespace watcher started");

    let deployment_watcher = watcher(api, ListParams::default()).try_for_each(|event| async {
        match event {
            kube::runtime::watcher::Event::Deleted(_namespace) => {
                // TODO: Don't unwatch deployments that aren't annotated.
            }
            kube::runtime::watcher::Event::Applied(_namespace) => {
                // TODO: Look for annotation removal and unwatch accordingly.
            }
            _ => {}
        }
        Ok(())
    });

    tokio::select! {
        res = deployment_watcher => {
            if let Err(e) = res {
                error!("kubernetes namespace watcher error: {}", e);
            }
        },
        _ = shutdown.recv() => { },
    };

    info!("kubernetes namespace watcher stopped");

    Ok(())
}
