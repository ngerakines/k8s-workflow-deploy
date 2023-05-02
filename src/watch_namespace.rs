use anyhow::Result;
use futures::prelude::*;
use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::{Api, ListParams, ResourceExt},
    runtime::watcher,
    Client,
};
use tokio::sync::broadcast::Receiver;
use tracing::{error, info};

use crate::{config::Settings, context::Context, k8s_util::annotation_true};

pub(crate) async fn watch_namespace(
    _settings: Settings,
    context: Context,
    shutdown: &mut Receiver<bool>,
) -> Result<()> {
    let client = Client::try_default().await.map_err(anyhow::Error::msg)?;
    let api = Api::<Namespace>::all(client.clone());

    info!("kubernetes namespace watcher started");

    for namespace in api.list(&ListParams::default()).await?.into_iter() {
        match annotation_true(
            namespace.annotations(),
            "workflow-deploy.ngerakines.me/enabled",
        ) {
            true => {
                if let Err(err) = context
                    .workflow_storage
                    .enable_namespace(namespace.name_any())
                    .await
                {
                    error!("Failed to enable namespace: {}", err);
                }
            }
            false => {
                if let Err(err) = context
                    .workflow_storage
                    .disable_namespace(namespace.name_any())
                    .await
                {
                    error!("Failed to disable namespace: {}", err);
                }
            }
        }
    }

    // There is a small, but real chance that in between the above list and the below watch, a namespace could be added, updated, or removed.

    let deployment_watcher = watcher(api, watcher::Config::default()).try_for_each(|event| async {
        match event {
            kube::runtime::watcher::Event::Deleted(namespace) => {
                context
                    .metrics
                    .count_with_tags("namespace_event.encountered", 1)
                    .with_tag("action", "deleted")
                    .with_tag("namespace_name", namespace.name_any().as_str())
                    .send();

                if let Err(err) = context
                    .workflow_storage
                    .disable_namespace(namespace.name_any())
                    .await
                {
                    error!("Failed to disable namespace: {}", err);
                }
            }
            kube::runtime::watcher::Event::Applied(namespace) => {
                context
                    .metrics
                    .count_with_tags("namespace_event.encountered", 1)
                    .with_tag("action", "applied")
                    .with_tag("namespace_name", namespace.name_any().as_str())
                    .send();

                match annotation_true(
                    namespace.annotations(),
                    "workflow-deploy.ngerakines.me/enabled",
                ) {
                    true => {
                        if let Err(err) = context
                            .workflow_storage
                            .enable_namespace(namespace.name_any())
                            .await
                        {
                            error!("Failed to enable namespace: {}", err);
                        }
                    }
                    false => {
                        if let Err(err) = context
                            .workflow_storage
                            .disable_namespace(namespace.name_any())
                            .await
                        {
                            error!("Failed to disable namespace: {}", err);
                        }
                    }
                }
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
