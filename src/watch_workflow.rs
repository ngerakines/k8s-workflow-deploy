use anyhow::Result;
use futures::prelude::*;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{Api, DeleteParams, PostParams, ResourceExt},
    runtime::watcher,
    Client, CustomResourceExt,
};
use tokio::sync::broadcast::Receiver;
use tracing::{error, info, log::warn};

use crate::crd::Workflow;
use crate::{config::Settings, context::Context};

pub(crate) async fn watch_workflow(
    _settings: Settings,
    context: Context,
    shutdown: &mut Receiver<bool>,
) -> Result<()> {
    let client = Client::try_default().await.map_err(anyhow::Error::msg)?;
    let api = Api::<Workflow>::all(client.clone());

    info!("kubernetes workflow watcher started");

    let deployment_watcher = watcher(api, watcher::Config::default()).try_for_each(|event| async {
        match event {
            kube::runtime::watcher::Event::Deleted(_workflow) => {
                warn!("Deleting workflows is not supported");
            }
            kube::runtime::watcher::Event::Applied(workflow) => {
                if let Err(err) = context
                    .workflow_storage
                    .add_workflow(workflow.clone())
                    .await
                {
                    error!("Failed to remove workflow: {}", err);
                }
            }
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

pub(crate) async fn init_workflow_crd() -> Result<()> {
    let client = Client::try_default().await.map_err(anyhow::Error::msg)?;
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());

    let dp = DeleteParams::default();
    let _ = crds
        .delete("workflow-deploy.ngerakines.me", &dp)
        .await
        .map(|res| {
            res.map_left(|o| {
                info!(
                    "Deleting {}: ({:?})",
                    o.name_any(),
                    o.status.unwrap().conditions.unwrap().last()
                );
            })
            .map_right(|s| {
                info!("Deleted workflow-deploy.ngerakines.me: ({:?})", s);
            })
        });

    let workflow_crd = Workflow::crd();

    let pp = PostParams::default();
    match crds.create(&pp, &workflow_crd).await {
        Ok(o) => {
            info!("Created {} ({:?})", o.name_any(), o.status.unwrap());
        }
        Err(kube::Error::Api(ae)) => {
            if ae.code != 409 {
                return Err(ae.into());
            }
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}
