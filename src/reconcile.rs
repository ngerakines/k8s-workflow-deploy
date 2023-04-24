use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use kube::api::ResourceExt;
use tokio::{
    sync::broadcast::Receiver,
    time::{sleep, Instant},
};
use tracing::{debug, error, info};

use crate::{action::Action, config::Settings, context::Context};

pub(crate) async fn reconcile_loop(
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
                debug!("Reconcile loop tick");
                let now = Utc::now();

                let workflows = context.workflow_storage.get_latest_workflows().await?;

                for workflow in workflows {
                    let workflow_name = workflow.name_any();
                    let reconcile_check = reconcile_checks.entry(workflow_name.clone()).or_insert_with(|| now);

                    if now > *reconcile_check {
                        info!("Reconciling workspace {workflow_name}");

                        if let Err(err) = context.action_tx.send(Action::ReconcileWorkflow(workflow.name_any(), now)).await {
                            error!("Failed to remove workflow: {}", err);
                        }

                        reconcile_checks.insert(workflow_name.clone(), now + Duration::seconds(5));
                    } else {
                        debug!("Skipping reconcile for {workflow_name}: {now} <= {reconcile_check}");
                    }
                }

                sleeper.as_mut().reset(Instant::now() + interval);
            }
        }
    }

    info!("reconcile loop ended");
    Ok(())
}
