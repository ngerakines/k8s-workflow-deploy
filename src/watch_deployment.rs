use anyhow::Result;
use futures::prelude::*;
use k8s_openapi::{api::apps::v1::Deployment, Resource};
use kube::{
    api::{Api, ResourceExt},
    runtime::watcher,
    Client,
};
use tokio::sync::broadcast::Receiver;
use tracing::{error, info};

use crate::{config::Settings, context::Context};

pub(crate) async fn watch_deployment(
    _settings: Settings,
    context: Context,
    shutdown: &mut Receiver<bool>,
) -> Result<()> {
    let client = Client::try_default().await.map_err(anyhow::Error::msg)?;
    let api = Api::<Deployment>::all(client.clone());

    let deployment_kind = format!("{};{}", Deployment::API_VERSION, Deployment::KIND);

    info!("kubernetes deployment watcher started");

    let deployment_watcher = watcher(api, watcher::Config::default()).try_for_each(|event| async {
        match event {
            kube::runtime::watcher::Event::Deleted(deployment) => {
                if let Err(err) = context
                    .workflow_storage
                    .remove_resource(
                        deployment.namespace().unwrap_or("default".to_string()),
                        deployment_kind.clone(),
                        deployment.name_any(),
                    )
                    .await
                {
                    error!("Failed to remove resource: {}", err);
                }
            }
            kube::runtime::watcher::Event::Applied(deployment) => {
                let namespace = deployment.namespace().unwrap_or("default".to_string());
                match deployment
                    .annotations()
                    .get("workflow-deploy.ngerakines.me/workflow")
                {
                    Some(workflow) => {
                        if let Err(err) = context
                            .workflow_storage
                            .add_resource(
                                namespace,
                                deployment_kind.clone(),
                                deployment.name_any(),
                                workflow.to_string(),
                                deployment.annotations().clone(),
                            )
                            .await
                        {
                            error!("Failed to add resource: {}", err);
                        }
                    }
                    None => {
                        if let Err(err) = context
                            .workflow_storage
                            .remove_resource(
                                namespace,
                                deployment_kind.clone(),
                                deployment.name_any(),
                            )
                            .await
                        {
                            error!("Failed to remove resource: {}", err);
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
                error!("kubernetes deployment watcher error: {}", e);
            }
        },
        _ = shutdown.recv() => { },
    };

    info!("kubernetes deployment watcher stopped");

    Ok(())
}
