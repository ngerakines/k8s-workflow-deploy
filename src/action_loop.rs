use std::{borrow::BorrowMut, collections::HashSet};

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use tokio::{
    sync::broadcast::Receiver,
    sync::mpsc::Receiver as ActionReceiver,
    time::{sleep, Instant},
};
use tracing::{debug, error, info, trace, warn};

use crate::{action::Action, config::Settings, context::Context, crd_storage::group_resources};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct WorkflowJob {
    workflow: String,
    checksum: u64,
    group: String,
    after: DateTime<Utc>,
    in_flight: bool,
}

impl WorkflowJob {
    fn should_retain(&self, workflow: &String) -> bool {
        if self.in_flight {
            return true;
        }

        if self.workflow != *workflow {
            return true;
        }

        false
    }
}

pub(crate) async fn action_loop(
    _settings: Settings,
    context: Context,
    shutdown: &mut Receiver<bool>,
    rx: &mut ActionReceiver<Action>,
) -> Result<()> {
    info!("action loop started");

    let one_second = Duration::seconds(1).to_std().unwrap();

    let sleeper = sleep(one_second);
    tokio::pin!(sleeper);

    // workflow_queue is a an unordered set of (workflow name, workflow checksum, group name, in flight) tuples. It is used to track which workflow deployment groups are in flight.
    let mut workflow_queue: HashSet<WorkflowJob> = HashSet::new();
    // let mut in_flight: HashMap<(String, String), JoinHandle<()>> = HashMap::new();

    'outer: loop {
        let now = Utc::now();

        tokio::select! {
            biased;
            _ = shutdown.recv() => {
                break 'outer;
            },
            () = &mut sleeper => {
                sleeper.as_mut().reset(Instant::now() + one_second);
                trace!("action loop timed out, resetting sleep");
            }
            r = rx.recv() => {
                // Nick: I'm not actually sure when this would happen. Something to look into.
                if r.is_none() {
                    continue 'outer;
                }

                let val = r.unwrap();
                debug!("action loop got value: {:?}", val);

                match val.clone() {
                    Action::WorkflowJobFinished(workflow_name, group) => {
                        // let handle_maybe = in_flight.get(&(workflow_name.clone(), group.clone()));
                        // if handle_maybe.is_none() {
                        //     error!("got workflow job finished for workflow: {:?} group: {:?} but no handle was found", workflow_name, group);
                        //     continue 'outer;
                        // }
                        // let handle: JoinHandle<()> = handle_maybe.unwrap();

                        workflow_queue.retain(|x| x.workflow != workflow_name && x.group != group);

                    }
                    Action::WorkflowUpdated(workflow_name, _) => {
                        // 1. Get the latest workflow checksum

                        let latest_workflow_res = context.workflow_storage.lastest_workflow(workflow_name.clone()).await;
                        if latest_workflow_res.is_err() {
                            error!("unable to get latest workflow for action: {:?}", val);
                            continue 'outer;
                        }
                        let latest_workflow = latest_workflow_res.unwrap();

                        // 2. Get all of the groups for the workflow

                        let workflow_res = context.workflow_storage.get_workflow(workflow_name.clone(), Some(latest_workflow)).await;
                        if workflow_res.is_err() {
                            error!("unable to get workflow version: {:?} {:?}", val, latest_workflow);
                            continue 'outer;
                        }
                        let workflow = workflow_res.unwrap();

                        let workflow_resources_res = context.workflow_storage.workflow_resources(workflow_name.clone()).await;
                        if workflow_resources_res.is_err() {
                            error!("unable to get workflow resources: {:?}", val);
                            continue 'outer;
                        }
                        let workflow_resources = workflow_resources_res.unwrap();

                        let resource_groups: HashSet<String> = group_resources(workflow, workflow_resources).iter().map(|x| x.1.clone()).collect::<HashSet<String>>();

                        // 3. Remove any items from the queue that are not in-flight and have the same workflow name and have a different workflow checksum
                        workflow_queue.retain(|x| x.should_retain(&workflow_name));

                        // 4. Add all of the groups to the queue
                        resource_groups.iter().for_each(|group| {
                            workflow_queue.insert(WorkflowJob {
                                workflow: workflow_name.clone(),
                                checksum: latest_workflow,
                                group: group.clone(),
                                after: now + Duration::seconds(15),
                                in_flight: false,
                            });
                        });

                    }
                    Action::ReconcileWorkflow(workflow, _) => {
                        // If there are any queued jobs, either in flight or waiting, for the workflow then don't do anything.
                        let queued_workflow_jobs = workflow_queue.iter().filter(|x| x.workflow == workflow).count();
                        if queued_workflow_jobs == 0 {
                            warn!("ReconcileWorkflow not implemented");
                        }
                    }
                }
            }
        }

        let workflow_names_res = context.workflow_storage.get_workflow_names();
        if workflow_names_res.is_err() {
            error!("unable to get workflow names: {:?}", workflow_names_res);
            continue 'outer;
        }
        let workflow_names = workflow_names_res.unwrap();

        for workflow_name in workflow_names {
            let max_in_flight = 3;
            let mut in_flight_count = max_in_flight
                - workflow_queue
                    .clone()
                    .iter()
                    .filter(|x| x.workflow == workflow_name && x.in_flight)
                    .count();

            'dispatch_queue: while in_flight_count > 0 {
                in_flight_count -= 1;

                let next_job_maybe = workflow_queue
                    .clone()
                    .into_iter()
                    .find(|x| x.workflow == workflow_name && !x.in_flight && x.after < now);
                if next_job_maybe.is_none() {
                    break 'dispatch_queue;
                }
                let next_job = next_job_maybe.unwrap();
                workflow_queue.borrow_mut().remove(&next_job);

                workflow_queue.borrow_mut().insert(WorkflowJob {
                    workflow: next_job.workflow.clone(),
                    checksum: next_job.checksum,
                    group: next_job.group.clone(),
                    after: next_job.after,
                    in_flight: true,
                });

                info!(
                    "dispatching job: {} {} {}",
                    next_job.workflow.clone(),
                    next_job.checksum,
                    next_job.group.clone()
                );

                {
                    let context = context.clone();
                    let next_job = next_job.clone();
                    tokio::spawn(async move {
                        if let Err(err) = action_workflow_updated(context, next_job).await {
                            error!(cause = ?err, "action_workflow_updated error");
                        }
                    })
                };
                // in_flight.insert(
                //     (next_job.workflow.clone(), next_job.group),
                //     group_join_handle,
                // );
            }
        }
    }

    info!("action loop ended");
    Ok(())
}

async fn action_workflow_updated(context: Context, workflow_job: WorkflowJob) -> Result<()> {
    info!("action_workflow_updated started");
    info!(
        "processing job: {} {} {}",
        workflow_job.workflow.clone(),
        workflow_job.checksum,
        workflow_job.group.clone()
    );

    if let Err(err) = context
        .action_tx
        .send(Action::WorkflowJobFinished(
            workflow_job.workflow.clone(),
            workflow_job.group.clone(),
        ))
        .await
    {
        error!(
            "Failed to notify that workflow updated job concluded: {}",
            err
        );
    }

    info!("action_workflow_updated ended");
    Ok(())
}
