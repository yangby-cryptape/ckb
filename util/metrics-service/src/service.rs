use std::time::Duration;

use metrics_runtime::Receiver;
use tokio::sync::oneshot;

use ckb_async_runtime::{new_runtime, Builder, Handle};
use ckb_stop_handler::{SignalSender, StopHandler};

use crate::Config;

#[must_use]
pub enum Guard {
    Off,
    On {
        handle: Handle,
        stop: StopHandler<()>,
    },
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Self::On { ref mut stop, .. } = self {
            stop.try_send();
        }
    }
}

pub fn init(config: Config) -> Result<Guard, String> {
    if config.exporter.is_empty() {
        return Ok(Guard::Off);
    }

    let mut runtime_builder = Builder::new();
    runtime_builder
        .threaded_scheduler()
        .enable_io()
        .enable_time();
    if config.threads != 0 {
        runtime_builder.core_threads(config.threads);
    } else {
        runtime_builder.core_threads(2);
    };
    let (signal_sender, mut signal_receiver) = oneshot::channel();
    let service = move |_: Handle| async move {
        loop {
            tokio::select! { _ = &mut signal_receiver => break }
        }
    };
    let (handle, thread) = new_runtime("Metrics", Some(runtime_builder), service);

    let receiver = {
        let histogram_window_secs = if config.histogram_window > 0 {
            config.histogram_window
        } else {
            10
        };
        let histogram_granularity_secs = if config.histogram_granularity > 0 {
            config.histogram_granularity
        } else {
            1
        };
        let upkeep_interval_millis = if config.upkeep_interval > 0 {
            config.upkeep_interval
        } else {
            50
        };
        let histogram_window = Duration::from_secs(histogram_window_secs);
        let histogram_granularity = Duration::from_secs(histogram_granularity_secs);
        let upkeep_interval = Duration::from_millis(upkeep_interval_millis);
        Receiver::builder()
            .histogram(histogram_window, histogram_granularity)
            .upkeep_interval(upkeep_interval)
    }
    .build()
    .unwrap();
    let controller = receiver.controller();

    for (name, exporter) in config.exporter {
        Config::check_exporter_name(&name)?;
        exporter.run(&handle, controller.clone())?;
    }

    receiver.install();

    let stop = StopHandler::new(SignalSender::Tokio(signal_sender), thread);

    Ok(Guard::On { handle, stop })
}
