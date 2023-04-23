use std::ops::Deref;
use std::sync::Arc;

use crate::config::Settings;
use crate::crd_storage::WorkflowStorage;

#[derive(Clone)]
pub(crate) struct Context(pub(crate) Arc<InnerContext>);

pub(crate) struct InnerContext {
    #[allow(unused)]
    pub(crate) settings: Settings,

    #[allow(unused)]
    pub(crate) workflow_storage: Box<dyn WorkflowStorage>,
}

impl InnerContext {
    pub(crate) fn new(settings: Settings, workflow_storage: Box<dyn WorkflowStorage>) -> Self {
        Self {
            settings,
            workflow_storage,
        }
    }
}

impl Deref for Context {
    type Target = InnerContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
