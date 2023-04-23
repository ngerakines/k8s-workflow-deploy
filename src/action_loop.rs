use anyhow::Result;
use chrono::{Duration, Utc};
use tokio::{
    sync::broadcast::Receiver,
    sync::mpsc::Receiver as ActionReceiver,
    time::{sleep, Instant},
};
use tracing::{debug, info, trace};

use crate::{action::Action, config::Settings, context::Context};

pub(crate) async fn action_loop(
    _settings: Settings,
    _context: Context,
    shutdown: &mut Receiver<bool>,
    rx: &mut ActionReceiver<Action>,
) -> Result<()> {
    info!("action loop started");

    let one_second = Duration::seconds(1).to_std().unwrap();

    let sleeper = sleep(one_second);
    tokio::pin!(sleeper);

    let mut work: Vec<Action> = Vec::new();
    let debounce_duration = chrono::Duration::seconds(10_i64);

    'outer: loop {
        tokio::select! {
            biased;
            _ = shutdown.recv() => {
                break 'outer;
            },
            () = &mut sleeper => {
                sleeper.as_mut().reset(Instant::now() + one_second);
                trace!("action loop timed out, resetting sleep");
            }
            r = rx.recv() => {
                let val = r.unwrap();
                debug!("action loop got value: {:?}", val);
                work.push(val);
            }
        }

        let now = Utc::now();

        let mut drained: Vec<Action> = vec![];
        for action in work.iter() {
            match action {
                Action::WorkflowUpdated(_, occurred) => {
                    if *occurred < now - debounce_duration {
                        info!("action loop draining: {:?}", action);
                        drained.push(action.clone());
                    }
                }
            }
        }

        for element in drained {
            debug!("action loop processing {:?}", element);
            work.retain(|x| x != &element);
        }
    }

    info!("action loop ended");
    Ok(())
}
