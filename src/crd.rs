use fnv::FnvHasher;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::hash::Hasher;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub(crate) struct WorkflowStepActionTarget {
    pub(crate) resource: String,
    pub(crate) name: String,
    pub(crate) containers: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub(crate) struct WorkflowStepAction {
    pub(crate) action: String,
    pub(crate) targets: Vec<WorkflowStepActionTarget>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub(crate) struct WorkflowStep {
    pub(crate) actions: Vec<WorkflowStepAction>,
}

#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "workflow-deploy.ngerakines.me",
    version = "v1alpha",
    kind = "Workflow",
    plural = "workflows"
)]
pub(crate) struct WorkflowSpec {
    pub(crate) namespaces: Vec<String>,
    pub(crate) version: String,
    pub(crate) debounce: Option<u32>,
    pub(crate) parallel: Option<u32>,
    pub(crate) supression: Vec<String>,
    pub(crate) steps: Vec<WorkflowStep>,
}

impl Workflow {
    pub(crate) fn checksum(&self) -> u64 {
        let mut hasher = FnvHasher::default();
        hasher.write(format!("version={}", self.spec.version).as_bytes());

        let mut namespaces = self.spec.namespaces.clone();
        namespaces.sort();
        for namespace in namespaces {
            hasher.write(format!("namespace={namespace}").as_bytes());
        }

        hasher.write(format!("debounce={}", self.spec.debounce.unwrap_or_default()).as_bytes());

        let mut supression = self.spec.supression.clone();
        supression.sort();
        for value in supression.iter() {
            hasher.write(format!("supression={}", value).as_bytes());
        }

        for step in self.spec.steps.iter() {
            hasher.write(format!("step={}", step.checksum()).as_bytes());
        }

        hasher.finish()
    }
}

impl WorkflowStep {
    pub(crate) fn checksum(&self) -> u64 {
        let mut hasher = FnvHasher::default();
        for action in self.actions.iter() {
            hasher.write(format!("step={}", action.checksum()).as_bytes());
        }
        hasher.finish()
    }
}

impl WorkflowStepAction {
    pub(crate) fn checksum(&self) -> u64 {
        let mut hasher = FnvHasher::default();
        for target in self.targets.iter() {
            hasher.write(format!("step={}", target.checksum()).as_bytes());
        }
        hasher.finish()
    }
}

impl WorkflowStepActionTarget {
    pub(crate) fn checksum(&self) -> u64 {
        let mut hasher = FnvHasher::default();
        hasher.write(format!("resource={} name={}", self.resource, self.name).as_bytes());
        let mut containers = self.containers.clone();
        containers.sort();
        for container in containers {
            hasher.write(format!("container={}", container).as_bytes());
        }
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_checksum() {
        let workflow = Workflow {
            metadata: Default::default(),
            spec: WorkflowSpec {
                version: "v1".to_string(),
                namespaces: vec!["default".to_string()],
                parallel: None,
                debounce: None,
                supression: vec![],
                steps: vec![],
            },
        };
        assert_eq!(workflow.checksum(), 8856693534762849072);
    }
}
