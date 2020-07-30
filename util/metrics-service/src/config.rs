use std::{collections::HashMap, net::SocketAddr, time::Duration};

use metrics_core::Observe;
use metrics_exporter_http::HttpExporter;
use metrics_exporter_log::LogExporter;
use metrics_observer_json::JsonBuilder;
use metrics_observer_prometheus::PrometheusBuilder;
use metrics_observer_yaml::YamlBuilder;
use regex::Regex;
use serde::{Deserialize, Serialize};

use ckb_async_runtime::Handle;

/* Examples:
 * ```toml
 * [metrics]
 * threads = 3
 * histogram_window = 60
 * histogram_granularity = 1
 * upkeep_interval = 500
 * [metrics.exporter.prometheus]
 * target = { type = "http", listen_address = "127.0.0.1:8100" }
 * format = { type = "prometheus" }
 * [metrics.exporter.log_yaml]
 * target = { type = "log", level = "warn", interval = 600 }
 * format = { type = "yaml" }
 * [metrics.exporter.log_json]
 * target = { type = "log", level = "error", interval = 900 }
 * format = { type = "json" }
 * ```
 */
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub threads: usize,

    #[serde(default)]
    pub histogram_window: u64, // seconds
    #[serde(default)]
    pub histogram_granularity: u64, // seconds
    #[serde(default)]
    pub upkeep_interval: u64, // milliseconds

    #[serde(default)]
    pub exporter: HashMap<String, Exporter>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Exporter {
    pub target: Target,
    pub format: Format,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum Target {
    Log {
        level: LogLevel,
        interval: u64, // seconds
    },
    Http {
        listen_address: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum Format {
    Json {
        #[serde(default)]
        pretty: bool,
    },
    Yaml,
    Prometheus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Config {
    pub fn check_exporter_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("the name of exporter shouldn't be empty".to_owned());
        }
        match Regex::new(r"^[0-9a-zA-Z_-]+$") {
            Ok(re) => {
                if !re.is_match(&name) {
                    return Err(format!(
                        "invaild exporter name \"{}\", only \"0-9a-zA-Z_-\" are allowed",
                        name
                    ));
                }
            }
            Err(err) => {
                return Err(format!("failed to check the name of exporter: {}", err));
            }
        }
        Ok(())
    }
}

impl Exporter {
    pub(crate) fn run<C>(self, handle: &Handle, c: C) -> Result<(), String>
    where
        C: Observe + Sync + Send + 'static,
    {
        let Self { target, format } = self;
        match target {
            Target::Log { level, interval } => {
                let lv: log::Level = level.into();
                let dur = Duration::from_secs(interval);
                match format {
                    Format::Json { pretty } => {
                        let b = JsonBuilder::new().set_pretty_json(pretty);
                        let exporter = LogExporter::new(c, b, lv, dur);
                        handle.spawn(async {
                            tokio::spawn(exporter.async_run());
                        });
                    }
                    Format::Yaml => {
                        let b = YamlBuilder::new();
                        let exporter = LogExporter::new(c, b, lv, dur);
                        handle.spawn(async {
                            tokio::spawn(exporter.async_run());
                        });
                    }
                    Format::Prometheus => {
                        let b = PrometheusBuilder::new();
                        let exporter = LogExporter::new(c, b, lv, dur);
                        handle.spawn(async {
                            tokio::spawn(exporter.async_run());
                        });
                    }
                };
            }
            Target::Http { listen_address } => {
                let addr = listen_address
                    .parse::<SocketAddr>()
                    .map_err(|err| format!("failed to parse listen_address because {}", err))?;
                match format {
                    Format::Json { pretty } => {
                        let b = JsonBuilder::new().set_pretty_json(pretty);
                        let exporter = HttpExporter::new(c, b, addr);
                        handle.spawn(async {
                            tokio::spawn(exporter.async_run());
                        });
                    }
                    Format::Yaml => {
                        let b = YamlBuilder::new();
                        let exporter = HttpExporter::new(c, b, addr);
                        handle.spawn(async {
                            tokio::spawn(exporter.async_run());
                        });
                    }
                    Format::Prometheus => {
                        let b = PrometheusBuilder::new();
                        let exporter = HttpExporter::new(c, b, addr);
                        handle.spawn(async {
                            tokio::spawn(exporter.async_run());
                        });
                    }
                };
            }
        }
        Ok(())
    }
}

impl From<LogLevel> for log::Level {
    fn from(lv: LogLevel) -> Self {
        match lv {
            LogLevel::Error => Self::Error,
            LogLevel::Warn => Self::Warn,
            LogLevel::Info => Self::Info,
            LogLevel::Debug => Self::Debug,
            LogLevel::Trace => Self::Trace,
        }
    }
}
