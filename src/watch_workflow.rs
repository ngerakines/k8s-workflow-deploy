use anyhow::Result;
use futures::prelude::*;
use kube::{
    api::{Api, ListParams},
    runtime::watcher,
    Client,
};
use tokio::sync::broadcast::Receiver;
use tracing::{error, info};

use crate::crd::Workflow;
use crate::{config::Settings, context::Context};

pub(crate) async fn watch_workflow(
    _settings: Settings,
    _context: Context,
    shutdown: &mut Receiver<bool>,
) -> Result<()> {
    let client = Client::try_default().await.map_err(anyhow::Error::msg)?;
    let api = Api::<Workflow>::all(client.clone());

    info!("kubernetes workflow watcher started");

    let deployment_watcher = watcher(api, ListParams::default()).try_for_each(|event| async {
        match event {
            kube::runtime::watcher::Event::Deleted(_workflow) => {}
            kube::runtime::watcher::Event::Applied(_workflow) => {}
            _ => {}
        }
        Ok(())
    });

    tokio::select! {
        res = deployment_watcher => {
            if let Err(e) = res {
                error!("kubernetes workflow watcher error: {}", e);
            }
        },
        _ = shutdown.recv() => { },
    };

    info!("kubernetes workflow watcher stopped");

    Ok(())
}
