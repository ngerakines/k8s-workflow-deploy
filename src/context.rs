use std::ops::Deref;
use std::panic::RefUnwindSafe;
use std::sync::Arc;

use cadence::MetricClient;
use tokio::sync::mpsc::Sender;

use crate::action::Action;
use crate::config::Settings;
use crate::crd_storage::WorkflowStorage;

#[derive(Clone)]
pub(crate) struct Context(pub(crate) Arc<InnerContext>);

pub(crate) struct InnerContext {
    #[allow(unused)]
    pub(crate) settings: Settings,
    pub(crate) workflow_storage: Box<dyn WorkflowStorage>,
    pub(crate) action_tx: Sender<Action>,
    pub(crate) metrics: Arc<dyn MetricClient + Send + Sync + RefUnwindSafe>,
}

impl InnerContext {
    pub(crate) fn new(
        settings: Settings,
        workflow_storage: Box<dyn WorkflowStorage>,
        action_tx: Sender<Action>,
        metrics: Arc<dyn MetricClient + Send + Sync + RefUnwindSafe>,
    ) -> Self {
        Self {
            settings,
            workflow_storage,
            action_tx,
            metrics,
        }
    }
}

impl Deref for Context {
    type Target = InnerContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
