use anyhow::Result;
use futures::prelude::*;
use k8s_openapi::{api::apps::v1::Deployment, Resource};
use kube::{
    api::{Api, ListParams, ResourceExt},
    runtime::watcher,
    Client,
};
use tokio::sync::broadcast::Receiver;
use tracing::{error, info};

use crate::context::Context;

pub(crate) async fn watch_deployment(
    context: Context,
    shutdown: &mut Receiver<bool>,
) -> Result<()> {
    let client = Client::try_default().await.map_err(anyhow::Error::msg)?;
    let api = Api::<Deployment>::all(client.clone());

    let deployment_kind = format!("{};{}", Deployment::API_VERSION, Deployment::KIND);

    info!("kubernetes deployment watcher started");

    for deployment in api.list(&ListParams::default()).await?.into_iter() {
        info!("deployment status: {:?}", deployment.status);
        let namespace = deployment.namespace().unwrap_or("default".to_string());

        let ready = deployment
            .clone()
            .status
            .map(|status| {
                status.conditions.iter().all(|conditions| {
                    conditions
                        .iter()
                        .all(|condition| condition.status == "True")
                })
            })
            .unwrap_or_default();
        info!("deployment ready: {}", ready);

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
                        ready,
                    )
                    .await
                {
                    error!("Failed to add resource: {}", err);
                }
            }
            None => {
                if let Err(err) = context
                    .workflow_storage
                    .remove_resource(namespace, deployment_kind.clone(), deployment.name_any())
                    .await
                {
                    error!("Failed to remove resource: {}", err);
                }
            }
        }
    }

    // There is a small, but real chance that in between the above list and the below watch, a deployment could be added, updated, or removed.

    let deployment_watcher = watcher(api, watcher::Config::default()).try_for_each(|event| async {
        match event {
            kube::runtime::watcher::Event::Deleted(deployment) => {
                context
                    .metrics
                    .count_with_tags("deployment_event.encountered", 1)
                    .with_tag("action", "deleted")
                    .with_tag("namespace_name", deployment.name_any().as_str())
                    .send();

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
                context
                    .metrics
                    .count_with_tags("deployment_event.encountered", 1)
                    .with_tag("action", "applied")
                    .with_tag("namespace_name", deployment.name_any().as_str())
                    .send();

                info!("deployment status: {:?}", deployment.status);
                let namespace = deployment.namespace().unwrap_or("default".to_string());

                let ready = deployment
                    .clone()
                    .status
                    .map(|status| {
                        status.conditions.iter().all(|conditions| {
                            conditions
                                .iter()
                                .all(|condition| condition.status == "True")
                        })
                    })
                    .unwrap_or_default();
                info!("deployment ready: {}", ready);

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
                                ready,
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
