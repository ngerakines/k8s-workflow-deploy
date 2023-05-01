#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum Action {
    WorkflowUpdated(String),
    ReconcileWorkflow(String),
    WorkflowJobFinished(String, String, bool),
}
