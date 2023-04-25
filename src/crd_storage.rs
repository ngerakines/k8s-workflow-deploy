use anyhow::{anyhow, Result};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::crd::Workflow;

// A known resource is deployment or job in a namespace that is associated with a workflow.
#[derive(Debug, Serialize, Deserialize, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub(crate) struct KnownResource {
    // The order of attributes matters.
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) workflow: String,
    pub(crate) annotations: BTreeMap<String, String>,
}

impl KnownResource {
    pub(crate) fn group_key(&self, keys: &Vec<String>) -> String {
        if keys.is_empty() {
            return self.namespace.clone();
        }
        let mut group_key = String::new();
        let mut sep = String::new();
        for key in keys {
            if key == "namespace" {
                group_key.push_str(&format!("{}{}", sep, self.namespace));
                sep = ",".to_string();
            } else if let Some(v) = self.annotations.get(key) {
                group_key.push_str(&format!("{}{}", sep, v));
                sep = ",".to_string();
            }
        }
        group_key
    }
}

#[async_trait]
pub(crate) trait WorkflowStorage: Sync + Send {
    async fn add_workflow(&self, workflow: Workflow) -> Result<()>;
    async fn lastest_workflow(&self, name: String) -> Result<u64>;
    async fn get_workflow(&self, name: String, checksum: Option<u64>) -> Result<Workflow>;
    async fn get_latest_workflows(&self) -> Result<Vec<Workflow>>;
    fn get_workflow_names(&self) -> Result<Vec<String>>;

    // Add a resource to the list of known resources.
    async fn add_resource(
        &self,
        namespace: String,
        kind: String,
        name: String,
        workflow: String,
        annotations: BTreeMap<String, String>,
    ) -> Result<()>;
    // Remove a resource from the list of known resources.
    async fn remove_resource(&self, namespace: String, kind: String, name: String) -> Result<()>;
    async fn workflow_resources(&self, workflow: String) -> Result<Vec<KnownResource>>;

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
    async fn add_workflow(&self, _workflow: Workflow) -> Result<()> {
        Ok(())
    }

    async fn lastest_workflow(&self, _name: String) -> Result<u64> {
        Ok(0)
    }

    async fn get_workflow(&self, _name: String, _checksum: Option<u64>) -> Result<Workflow> {
        Err(anyhow!("not found"))
    }

    async fn get_latest_workflows(&self) -> Result<Vec<Workflow>> {
        Ok(vec![])
    }

    fn get_workflow_names(&self) -> Result<Vec<String>> {
        Ok(vec![])
    }

    async fn add_resource(
        &self,
        _namespace: String,
        _kind: String,
        _name: String,
        _workflow: String,
        _annotations: BTreeMap<String, String>,
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

    async fn workflow_resources(&self, _workflow: String) -> Result<Vec<KnownResource>> {
        Ok(vec![])
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
    async fn add_workflow(&self, workflow: Workflow) -> Result<()> {
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

    async fn lastest_workflow(&self, name: String) -> Result<u64> {
        let inner_lock = self.inner.lock();
        let inner = inner_lock.borrow_mut();

        if let Some(checksum) = inner.latest.get(&name) {
            return Ok(*checksum);
        }

        Err(anyhow!("not found"))
    }

    async fn get_workflow(&self, name: String, checksum: Option<u64>) -> Result<Workflow> {
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

    async fn get_latest_workflows(&self) -> Result<Vec<Workflow>> {
        let inner_lock = self.inner.lock();
        let inner = inner_lock.borrow_mut();

        Ok(inner
            .latest
            .values()
            .cloned()
            .map(|c| inner.workflows.get(&c).unwrap().clone())
            .collect())
    }

    fn get_workflow_names(&self) -> Result<Vec<String>> {
        let inner_lock = self.inner.lock();
        let inner = inner_lock.borrow_mut();

        Ok(inner.latest.keys().cloned().collect())
    }

    async fn add_resource(
        &self,
        namespace: String,
        kind: String,
        name: String,
        workflow: String,
        annotations: BTreeMap<String, String>,
    ) -> Result<()> {
        let inner_lock = self.inner.lock();
        let mut inner = inner_lock.borrow_mut();

        inner.resources.insert(KnownResource {
            namespace,
            kind,
            name,
            workflow,
            annotations,
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

    async fn workflow_resources(&self, workflow: String) -> Result<Vec<KnownResource>> {
        let inner_lock = self.inner.lock();
        let inner = inner_lock.borrow_mut();

        Ok(inner
            .resources
            .iter()
            .filter(|r| r.workflow == workflow)
            .cloned()
            .collect())
    }
}

pub(crate) fn get_workflow_storage(workflow_storage_type: &str) -> Box<dyn WorkflowStorage> {
    match workflow_storage_type {
        #[cfg(debug_assertions)]
        "null" => Box::<NullWorkflowStorager>::default() as Box<dyn WorkflowStorage>,

        "memory" => Box::<MemoryWorkflowStorager>::default() as Box<dyn WorkflowStorage>,

        _ => panic!("Unknown workflow storage type: {workflow_storage_type}"),
    }
}

#[allow(unused)]
pub(crate) fn group_resources(
    workflow: Workflow,
    resources: Vec<KnownResource>,
) -> Vec<(u32, String, KnownResource)> {
    let group_annotations = workflow.spec.group_annotations.unwrap_or_default();
    let mut results = HashSet::new();
    for resource in resources {
        results.insert((1, resource.group_key(&group_annotations), resource));
    }
    let mut groups: Vec<(u32, String, KnownResource)> = results.into_iter().collect();
    groups.sort();
    groups
}

#[cfg(test)]
mod tests {
    use crate::crd::WorkflowSpec;

    use super::*;

    #[tokio::test]
    async fn test_group_resources() {
        let default_workflow = Workflow {
            metadata: Default::default(),
            spec: WorkflowSpec {
                version: "v1".to_string(),
                group_annotations: None,
                debounce: None,
                supression: None,
                steps: None,
            },
        };
        let namespaced_subgroups_workflow = Workflow {
            metadata: Default::default(),
            spec: WorkflowSpec {
                version: "v1".to_string(),
                group_annotations: Some(vec!["namespace".to_string(), "subgroup".to_string()]),
                debounce: None,
                supression: None,
                steps: None,
            },
        };
        let subgroups_workflow = Workflow {
            metadata: Default::default(),
            spec: WorkflowSpec {
                version: "v1".to_string(),
                group_annotations: Some(vec!["subgroup".to_string()]),
                debounce: None,
                supression: None,
                steps: None,
            },
        };
        let resources = vec![
            KnownResource {
                namespace: "foo".to_string(),
                kind: "Deployment".to_string(),
                name: "api".to_string(),
                workflow: "default".to_string(),
                annotations: BTreeMap::from([("subgroup".to_string(), "main".to_string())]),
            },
            KnownResource {
                namespace: "foo".to_string(),
                kind: "Deployment".to_string(),
                name: "worker".to_string(),
                workflow: "default".to_string(),
                annotations: BTreeMap::from([("subgroup".to_string(), "main".to_string())]),
            },
            KnownResource {
                namespace: "foo".to_string(),
                kind: "Deployment".to_string(),
                name: "canary-api".to_string(),
                workflow: "default".to_string(),
                annotations: BTreeMap::from([("subgroup".to_string(), "canary".to_string())]),
            },
        ];
        assert_eq!(
            group_resources(default_workflow, resources.clone()),
            vec![
                (1, "foo".to_string(), resources[0].clone()),
                (1, "foo".to_string(), resources[2].clone()),
                (1, "foo".to_string(), resources[1].clone())
            ]
        );
        assert_eq!(
            group_resources(namespaced_subgroups_workflow, resources.clone()),
            vec![
                (1, "foo,canary".to_string(), resources[2].clone()),
                (1, "foo,main".to_string(), resources[0].clone()),
                (1, "foo,main".to_string(), resources[1].clone())
            ]
        );
        assert_eq!(
            group_resources(subgroups_workflow, resources.clone()),
            vec![
                (1, "canary".to_string(), resources[2].clone()),
                (1, "main".to_string(), resources[0].clone()),
                (1, "main".to_string(), resources[1].clone())
            ]
        );
    }
}
