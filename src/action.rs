use chrono::{DateTime, Utc};

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum Action {
    WorkflowUpdated(String, DateTime<Utc>),
    ReconcileWorkflow(String, DateTime<Utc>),
}
