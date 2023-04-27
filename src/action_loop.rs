use std::{borrow::BorrowMut, collections::HashSet};

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    api::{Patch, PatchParams},
    Api, Client,
};
use tokio::{
    sync::broadcast::Receiver,
    sync::mpsc::Receiver as ActionReceiver,
    time::{sleep, Instant},
};
use tracing::{debug, error, info, trace, warn};

use crate::{action::Action, config::Settings, context::Context};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct WorkflowJob {
    workflow: String,
    checksum: u64,
    group: String,
    after: DateTime<Utc>,
    in_flight: bool,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum WorkflowAction {
    Started(),
    UpdateDeployment(String, Vec<(String, String)>),
    WaitDeploymentReady(String),
}

impl WorkflowJob {
    fn should_retain(&self, workflow: &String) -> bool {
        self.in_flight || self.workflow != *workflow
    }
}

pub(crate) async fn action_loop(
    _settings: Settings,
    context: Context,
    shutdown: &mut Receiver<bool>,
    rx: &mut ActionReceiver<Action>,
) -> Result<()> {
    info!("action loop started");

    let one_second = Duration::seconds(3).to_std().unwrap();

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

                        // 3. Remove any items from the queue that are not in-flight and have the same workflow name and have a different workflow checksum
                        workflow_queue.retain(|x| x.should_retain(&workflow_name));

                        // 4. Add all of the groups to the queue
                        workflow.spec.namespaces.iter().for_each(|namespace| {
                            workflow_queue.insert(WorkflowJob {
                                workflow: workflow_name.clone(),
                                checksum: latest_workflow,
                                group: namespace.clone(),
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

    let workflow = context
        .workflow_storage
        .get_workflow(workflow_job.workflow.clone(), Some(workflow_job.checksum))
        .await?;

    let one_second = Duration::seconds(1).to_std().unwrap();

    let sleeper = sleep(one_second);
    tokio::pin!(sleeper);

    let mut work_queue: Vec<WorkflowAction> = vec![];

    // Nick: My thinking is that it's easier to create a big list of everything
    // that needs to be done for a workflow in the context of a group
    // (namespace) up front. The alternative would be to parse the workflow
    // spec every loop to see what's next. The added bonus of doing it this way
    // is that I can also populate history as each thing is completed.
    for step in workflow.spec.steps {
        for action in step.actions {
            if action.action == *"update_deployment" {
                for target in &action.targets {
                    work_queue.push(WorkflowAction::UpdateDeployment(
                        target.name.clone(),
                        target
                            .containers
                            .iter()
                            .map(|container| (container.clone(), workflow.spec.version.clone()))
                            .collect(),
                    ));
                }
                for target in &action.targets {
                    work_queue.push(WorkflowAction::WaitDeploymentReady(target.name.clone()));
                }
            }
        }
    }

    let mut history: Vec<(WorkflowAction, DateTime<Utc>)> =
        vec![(WorkflowAction::Started(), Utc::now())];

    let client = Client::try_default()
        .await
        .map_err(anyhow::Error::msg)
        .unwrap();

    let deployment_client: Api<Deployment> =
        Api::namespaced(client.clone(), &workflow_job.group.clone());

    'working: loop {
        tokio::select! {
            () = &mut sleeper => {

                if work_queue.is_empty() {
                    info!("action_workflow_updated queue is empty");
                    break 'working;
                }

                let now = Utc::now();

                // match history[0].clone() {
                //     (WorkflowAction::Started(), _) => {}
                //     (WorkflowAction::WaitDeploymentReady(_), _) => { }
                //     (WorkflowAction::UpdateDeployment(deployment_name, _), occurred_at) => {
                //         // TODO: Make this configurable.
                //         if now > occurred_at + Duration::seconds(3) {
                //             info!("Waiting for more time to pass after updating deployment {}", &deployment_name);
                //             sleeper.as_mut().reset(Instant::now() + one_second);
                //             continue 'working;
                //         }
                //     }
                // }

                match work_queue[0] {
                    WorkflowAction::Started() => {
                        work_queue.remove(0);
                    }
                    WorkflowAction::UpdateDeployment(ref name, ref containers) => {
                        info!("action_workflow_updated UpdateDeployment: {}", name);

                        // TODO: Update the deployment.
                        let deployment = deployment_client.get_opt(name).await;
                        if let Err(err) = deployment {
                            error!("UpdateDeployment unable to get deployment {}: {}", name, err);
                            break 'working;
                        }
                        let deployment = deployment.unwrap();
                        if deployment.is_none() {
                            error!("UpdateDeployment unable to get deployment {}: not found", name);
                            break 'working;
                        }

                        let mut json_patch = json_patch::Patch(vec![]);
                        for (index, container) in deployment.unwrap_or_default().spec.unwrap_or_default().template.spec.unwrap_or_default().containers.iter().enumerate() {
                            info!("container: {}", container.name);
                            if let Some(version) = containers.iter().find(|x| x.0 == container.name).map(|x| x.1.clone()) {
                                json_patch.0.push(json_patch::PatchOperation::Replace(
                                    json_patch::ReplaceOperation{
                                        path: format!("/spec/template/spec/containers/{index}/image"),
                                        value:serde_json::to_value(version).unwrap()
                                    },
                                ));
                            }
                        }

                        let patch_res = deployment_client
                        .patch(
                            name,
                            &PatchParams::default(),
                            &Patch::Json::<()>(json_patch),
                        )
                        .await;
                        if let Err(err) = patch_res {
                            error!("UpdateDeployment patching deployment {} failed: {}", name, err);
                            break 'working;
                        }

                        history.push((work_queue[0].clone(), now));
                        work_queue.remove(0);
                    }
                    WorkflowAction::WaitDeploymentReady(ref name) => {
                        info!("action_workflow_updated WaitDeploymentReady: {}", name);

                        let last_deployed_at = history.iter().rev().find(|x| match x.0 { WorkflowAction::UpdateDeployment(ref update_deployment_name, _) => update_deployment_name == name, _ => false }).map(|x| x.1);
                        if last_deployed_at.is_none() {
                            error!("WaitDeploymentReady failed: No deployment found for {}", name);
                            break 'working;
                        }
                        let last_deployed_at = last_deployed_at.unwrap();

                        // 1. If we aren't ready to wait yet then continue
                        if now < last_deployed_at + Duration::seconds(5) {
                            info!("Waiting for more time to pass after updating deployment {}", &name);
                            sleeper.as_mut().reset(Instant::now() + one_second);
                            continue 'working;
                        }

                        // 2. Get the status of the deployment
                        let deployment_is_ready = true;

                        // 3. Continue if the status is not ready and we have not reached the max wait time
                        if !deployment_is_ready && now < last_deployed_at + Duration::seconds(90) {
                            info!("Waiting for more time to pass after updating deployment {}", &name);
                            sleeper.as_mut().reset(Instant::now() + one_second);
                            continue 'working;
                        }

                        // 4. Error if the status is not ready and we have passed the max wait time
                        if !deployment_is_ready && now > last_deployed_at + Duration::seconds(30) {
                            error!("WaitDeploymentReady failed: Deployment {} did not become ready within wait period", name);
                            sleeper.as_mut().reset(Instant::now() + one_second);
                            break 'working;
                        }

                        history.push((work_queue[0].clone(), now));
                        work_queue.remove(0);
                    }
                }

                sleeper.as_mut().reset(Instant::now() + one_second);
                trace!("action_workflow_updated tick");
            }
        }
    }

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
