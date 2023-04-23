use fnv::FnvHasher;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::hash::Hasher;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub(crate) struct SupressionRange {
    pub(crate) start: String,
    pub(crate) end: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub(crate) struct WorkflowStepActionTarget {
    pub(crate) resource: String,
    pub(crate) name: String,
    pub(crate) containers: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub(crate) struct WorkflowStepAction {
    pub(crate) action: String,
    pub(crate) position: Option<u32>,
    pub(crate) targets: Option<Vec<WorkflowStepActionTarget>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub(crate) struct WorkflowStep {
    pub(crate) position: Option<u32>,
    pub(crate) actions: Option<Vec<WorkflowStepAction>>,
}

#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "workflow-deploy.ngerakines.me",
    version = "v1alpha",
    kind = "Workflow",
    plural = "workflows"
)]
pub(crate) struct WorkflowSpec {
    pub(crate) version: String,
    pub(crate) group_by_namespace: Option<bool>,
    pub(crate) group_annotations: Option<Vec<String>>,
    pub(crate) debounce: Option<u32>,
    pub(crate) supression: Option<Vec<SupressionRange>>,
    pub(crate) steps: Option<Vec<WorkflowStep>>,
}

impl Workflow {
    #[allow(unused)]
    pub(crate) fn checksum(&self) -> u64 {
        let mut hasher = FnvHasher::default();
        hasher.write(format!("version={}", self.spec.version).as_bytes());
        hasher.write(
            format!(
                "group_by_namespace={}",
                self.spec.group_by_namespace.unwrap_or_default()
            )
            .as_bytes(),
        );
        for annotation in self.spec.group_annotations.as_ref().unwrap_or(&vec![]) {
            hasher.write(format!("group_annotations={}", annotation).as_bytes());
        }
        hasher.write(format!("debounce={}", self.spec.debounce.unwrap_or_default()).as_bytes());
        for supression in self.spec.supression.as_ref().unwrap_or(&vec![]) {
            hasher.write(format!("supression={}", supression.checksum()).as_bytes());
        }
        for step in self.spec.steps.as_ref().unwrap_or(&vec![]) {
            hasher.write(format!("step={}", step.checksum()).as_bytes());
        }
        hasher.finish()
    }
}

impl SupressionRange {
    #[allow(unused)]
    pub(crate) fn checksum(&self) -> u64 {
        let mut hasher = FnvHasher::default();
        hasher.write(
            format!(
                "start={} end={}",
                self.start,
                self.end.clone().unwrap_or_default()
            )
            .as_bytes(),
        );
        hasher.finish()
    }
}

impl WorkflowStep {
    #[allow(unused)]
    pub(crate) fn checksum(&self) -> u64 {
        let mut hasher = FnvHasher::default();
        hasher.write(format!("position={}", self.position.unwrap_or_default()).as_bytes());
        for action in self.actions.as_ref().unwrap_or(&vec![]) {
            hasher.write(format!("step={}", action.checksum()).as_bytes());
        }
        hasher.finish()
    }
}

impl WorkflowStepAction {
    #[allow(unused)]
    pub(crate) fn checksum(&self) -> u64 {
        let mut hasher = FnvHasher::default();
        hasher.write(format!("position={}", self.position.unwrap_or_default()).as_bytes());
        for target in self.targets.as_ref().unwrap_or(&vec![]) {
            hasher.write(format!("step={}", target.checksum()).as_bytes());
        }
        hasher.finish()
    }
}

impl WorkflowStepActionTarget {
    #[allow(unused)]
    pub(crate) fn checksum(&self) -> u64 {
        let mut hasher = FnvHasher::default();
        hasher.write(format!("resource={} name={}", self.resource, self.name).as_bytes());
        let mut containers = self.containers.clone().unwrap_or(vec![]);
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
                group_by_namespace: None,
                group_annotations: None,
                debounce: None,
                supression: None,
                steps: None,
            },
        };
        assert_eq!(workflow.checksum(), 9021128205704642950);
    }
}
