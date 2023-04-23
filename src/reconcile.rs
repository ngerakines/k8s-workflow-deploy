use std::{
    collections::HashMap,
};

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use tokio::{
    sync::broadcast::Receiver,
    time::{sleep, Instant},
};
use tracing::{error, info, trace, debug};

use futures::prelude::*;
use k8s_openapi::{api::apps::v1::Deployment, Resource};
use kube::{
    api::{Api, ResourceExt},
    runtime::watcher,
    Client,
};

use crate::{config::Settings, context::Context, crd::Workflow};


pub (crate) async fn reconcile_loop(
    _settings: Settings,
    context: Context,
    shutdown: &mut Receiver<bool>,
) -> Result<()> {

    info!("reconcile loop started");

    let interval = Duration::seconds(5).to_std()?;

    let sleeper = sleep(interval);
    tokio::pin!(sleeper);

    let mut reconcile_checks: HashMap<String, DateTime<Utc>> = HashMap::new();

    'outer: loop {
        tokio::select! {
            _ = shutdown.recv() => {
                break 'outer;
            },
            () = &mut sleeper => {
                info!("Reconcile loop tick");
                let now = Utc::now();

                let workflows = context.workflow_storage.get_latest_workflows().await?;

                for workflow in workflows {
                    let workflow_name = workflow.name_any();
                    let reconcile_check = reconcile_checks.entry(workflow_name.clone()).or_insert_with(|| now);

                    if now > *reconcile_check {
                        info!("Reconciling workspace {workflow_name}");

                        *reconcile_check = now + Duration::seconds(5);
                    } else {
                        debug!("Skipping reconcile for {workflow_name}: {now} <= {reconcile_check}");
                    }
                }

                // for (key, value) in &keys {
                //     trace!("Preparing {key} {value}");
                //     if now > *value {
                //         info!("Countdown expired: {key}");

                //         if let Err(err) = client.gauge_with_tags(&config.countdown_key, 0).with_tag(&config.countdown_id, key).try_send() {
                //             error!(cause = ?err, "Error sending metric");
                //         }

                //         continue
                //     }
                //     let remaining = (*value - now).num_seconds();
                //     if let Err(err) = client.gauge_with_tags(&config.countdown_key, remaining as u64).with_tag(&config.countdown_id, key).try_send() {
                //         error!(cause = ?err, "Error sending metric");
                //     }
                // }

                // if let Err(err) = client.gauge(&config.heartbeat_metric, now.timestamp() as u64) {
                //     error!(cause = ?err, "Error sending heartbeat metric");
                // }

                sleeper.as_mut().reset(Instant::now() + interval);
            }
        }
    }

    info!("reconcile loop ended");
    Ok(())
}
