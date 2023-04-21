use anyhow::{anyhow, Result};
use async_trait::async_trait;
use parking_lot::Mutex;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::crd::Workflow;

#[async_trait]
pub(crate) trait WorkflowStorage: Sync + Send {
    async fn add(&self, workflow: Workflow) -> Result<bool>;
    async fn lastest(&self, name: String) -> Result<u64>;
    async fn get(&self, name: String, checksum: Option<u64>) -> Result<Workflow>;
}

#[derive(Default)]
pub struct NullWorkflowStorager;

#[async_trait]
impl WorkflowStorage for NullWorkflowStorager {
    async fn add(&self, _workflow: Workflow) -> Result<bool> {
        Ok(true)
    }

    async fn lastest(&self, _name: String) -> Result<u64> {
        Ok(0)
    }

    async fn get(&self, _name: String, _checksum: Option<u64>) -> Result<Workflow> {
        Err(anyhow!("not found"))
    }
}

#[derive(Default)]
struct InnerMemoryWorkflowStorager {
    // TODO: Index on name and date
    workflows: HashMap<u64, Workflow>,
    latest: HashMap<String, u64>,
}

#[derive(Default)]
pub struct MemoryWorkflowStorager {
    inner: Mutex<RefCell<InnerMemoryWorkflowStorager>>,
}

#[async_trait]
impl WorkflowStorage for MemoryWorkflowStorager {
    async fn add(&self, workflow: Workflow) -> Result<bool> {
        let inner_lock = self.inner.lock();
        let mut inner = inner_lock.borrow_mut();

        if workflow.metadata.name.is_none() {
            return Err(anyhow!("workflow name is required"));
        }
        let name = workflow.metadata.name.clone().unwrap();
        let checksum = workflow.checksum();

        inner.workflows.insert(checksum, workflow);
        inner.latest.insert(name, checksum);

        Ok(true)
    }

    async fn lastest(&self, name: String) -> Result<u64> {
        let inner_lock = self.inner.lock();
        let inner = inner_lock.borrow_mut();

        if let Some(checksum) = inner.latest.get(&name) {
            return Ok(*checksum);
        }

        Err(anyhow!("not found"))
    }

    async fn get(&self, name: String, checksum: Option<u64>) -> Result<Workflow> {
        let inner_lock = self.inner.lock();
        let inner = inner_lock.borrow_mut();

        match checksum {
            Some(checksum) => {
                if let Some(workflow) = inner.workflows.get(&checksum) {
                    return Ok(workflow.clone());
                }
                Err(anyhow!("not found"))
            }
            None => {
                if let Some(checksum) = inner.latest.get(&name) {
                    if let Some(workflow) = inner.workflows.get(checksum) {
                        return Ok(workflow.clone());
                    }
                }
                Err(anyhow!("not found"))
            }
        }
    }
}

#[allow(unused)]
pub(crate) fn get_workflow_storage(workflow_storage_type: &str) -> Box<dyn WorkflowStorage> {
    match workflow_storage_type {
        #[cfg(debug_assertions)]
        "null" => Box::<NullWorkflowStorager>::default() as Box<dyn WorkflowStorage>,

        "memory" => Box::<MemoryWorkflowStorager>::default() as Box<dyn WorkflowStorage>,

        _ => panic!("Unknown workflow storage type: {workflow_storage_type}"),
    }
}
