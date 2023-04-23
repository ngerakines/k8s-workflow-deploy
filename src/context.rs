use std::ops::Deref;
use std::sync::Arc;

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
}

impl InnerContext {
    pub(crate) fn new(
        settings: Settings,
        workflow_storage: Box<dyn WorkflowStorage>,
        action_tx: Sender<Action>,
    ) -> Self {
        Self {
            settings,
            workflow_storage,
            action_tx,
        }
    }
}

impl Deref for Context {
    type Target = InnerContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
