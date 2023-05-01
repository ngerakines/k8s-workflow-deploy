use anyhow::{anyhow, Result};
use std::net::{ToSocketAddrs, UdpSocket};
use tracing::error;

use cadence::{MetricError, NopMetricSink, QueuingMetricSink, StatsdClient, UdpMetricSink};

use crate::config::Settings;

fn error_handler(err: MetricError) {
    error!("Metric error! {}", err);
}

pub(crate) fn metrics_client(settings: Settings) -> Result<StatsdClient> {
    if !settings.stats.enabled {
        return Ok(StatsdClient::from_sink("", NopMetricSink));
    }

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_nonblocking(true)?;

    let host = settings
        .stats
        .statsd_sink
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| {
            anyhow!(
                "Unable to resolve statsd sink {}",
                settings.stats.statsd_sink
            )
        })?;

    let udp_sink = UdpMetricSink::from(host, socket)?;
    let queuing_sink = QueuingMetricSink::from(udp_sink);
    let mut client_builder = StatsdClient::builder(&settings.stats.metric_prefix, queuing_sink)
        .with_error_handler(error_handler);
    for (k, v) in settings.stats.global_tags {
        client_builder = client_builder.with_tag(k, v);
    }
    Ok(client_builder.build())
}
