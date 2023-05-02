#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum Action {
    WorkflowUpdated(String, bool),
    ReconcileWorkflow(String),
    WorkflowJobFinished(String, String, bool),
}
