use anyhow::{anyhow, Result};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use crate::crd::Workflow;

// A known resource is deployment or job in a namespace that is associated with a workflow.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub(crate) struct KnownResource {
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) workflow: String,
}

#[async_trait]
pub(crate) trait WorkflowStorage: Sync + Send {
    async fn add_workspace(&self, workflow: Workflow) -> Result<()>;
    async fn lastest_workspace(&self, name: String) -> Result<u64>;
    async fn get_workspace(&self, name: String, checksum: Option<u64>) -> Result<Workflow>;

    // Add a resource to the list of known resources.
    async fn add_resource(
        &self,
        namespace: String,
        kind: String,
        name: String,
        workflow: String,
    ) -> Result<()>;
    // Remove a resource from the list of known resources.
    async fn remove_resource(&self, namespace: String, kind: String, name: String) -> Result<()>;

    // Add a namespace to the list of namespaces that are enabled.
    async fn enable_namespace(&self, name: String) -> Result<()>;
    // Remove a namespace from the list of namespaces that are enabled.
    async fn disable_namespace(&self, name: String) -> Result<()>;
    // Check if a namespace is enabled. This will be called whenever a known resource has an action.
    async fn namespace_enabled(&self, name: String) -> Result<bool>;
}

#[derive(Default)]
pub(crate) struct NullWorkflowStorager;

#[async_trait]
impl WorkflowStorage for NullWorkflowStorager {
    async fn add_workspace(&self, _workflow: Workflow) -> Result<()> {
        Ok(())
    }

    async fn lastest_workspace(&self, _name: String) -> Result<u64> {
        Ok(0)
    }

    async fn get_workspace(&self, _name: String, _checksum: Option<u64>) -> Result<Workflow> {
        Err(anyhow!("not found"))
    }

    async fn add_resource(
        &self,
        _namespace: String,
        _kind: String,
        _name: String,
        _workflow: String,
    ) -> Result<()> {
        Ok(())
    }

    async fn remove_resource(
        &self,
        _namespace: String,
        _kind: String,
        _name: String,
    ) -> Result<()> {
        Ok(())
    }

    async fn enable_namespace(&self, _name: String) -> Result<()> {
        Ok(())
    }

    async fn disable_namespace(&self, _name: String) -> Result<()> {
        Ok(())
    }

    async fn namespace_enabled(&self, _name: String) -> Result<bool> {
        Ok(true)
    }
}

#[derive(Default)]
struct InnerMemoryWorkflowStorager {
    workflows: HashMap<u64, Workflow>,
    latest: HashMap<String, u64>,

    resources: HashSet<KnownResource>,
    namespaces: HashSet<String>,
}

#[derive(Default)]
pub(crate) struct MemoryWorkflowStorager {
    inner: Mutex<RefCell<InnerMemoryWorkflowStorager>>,
}

#[async_trait]
impl WorkflowStorage for MemoryWorkflowStorager {
    async fn add_workspace(&self, workflow: Workflow) -> Result<()> {
        let inner_lock = self.inner.lock();
        let mut inner = inner_lock.borrow_mut();

        if workflow.metadata.name.is_none() {
            return Err(anyhow!("workflow name is required"));
        }
        let name = workflow.metadata.name.clone().unwrap();
        let checksum = workflow.checksum();

        inner.workflows.insert(checksum, workflow);
        inner.latest.insert(name, checksum);

        Ok(())
    }

    async fn lastest_workspace(&self, name: String) -> Result<u64> {
        let inner_lock = self.inner.lock();
        let inner = inner_lock.borrow_mut();

        if let Some(checksum) = inner.latest.get(&name) {
            return Ok(*checksum);
        }

        Err(anyhow!("not found"))
    }

    async fn get_workspace(&self, name: String, checksum: Option<u64>) -> Result<Workflow> {
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

    async fn add_resource(
        &self,
        namespace: String,
        kind: String,
        name: String,
        workflow: String,
    ) -> Result<()> {
        let inner_lock = self.inner.lock();
        let mut inner = inner_lock.borrow_mut();

        inner.resources.insert(KnownResource {
            namespace,
            kind,
            name,
            workflow,
        });

        Ok(())
    }

    async fn remove_resource(&self, namespace: String, kind: String, name: String) -> Result<()> {
        let inner_lock = self.inner.lock();
        let mut inner = inner_lock.borrow_mut();

        inner
            .resources
            .retain(|r| !(r.namespace == namespace && r.kind == kind && r.name == name));

        Ok(())
    }

    async fn enable_namespace(&self, name: String) -> Result<()> {
        let inner_lock = self.inner.lock();
        let mut inner = inner_lock.borrow_mut();

        inner.namespaces.insert(name);
        Ok(())
    }

    async fn disable_namespace(&self, name: String) -> Result<()> {
        let inner_lock = self.inner.lock();
        let mut inner = inner_lock.borrow_mut();

        inner.namespaces.remove(&name);

        Ok(())
    }

    async fn namespace_enabled(&self, name: String) -> Result<bool> {
        let inner_lock = self.inner.lock();
        let inner = inner_lock.borrow_mut();

        Ok(inner.namespaces.contains(&name))
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
