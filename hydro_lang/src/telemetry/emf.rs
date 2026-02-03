//! AWS CloudWatch embedded metric format (EMF).
//!
//! <https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/CloudWatch_Embedded_Metric_Format_Specification.html>
use std::marker::Unpin;
use std::panic::AssertUnwindSafe;
use std::time::{Duration, SystemTime};

use dfir_rs::Never;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::scheduled::metrics::DfirMetrics;
use futures::FutureExt;
use quote::quote;
use serde_json::json;
use syn::parse_quote;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio_metrics::RuntimeMetrics;

use crate::location::{LocationKey, LocationType};
use crate::staging_util::get_this_crate;
use crate::compile::deploy::Sidecar;

/// Default file path for [`RecordMetricsSidecar`].
pub const DEFAULT_FILE_PATH: &str = "/var/log/hydro/metrics.log";
/// Default interval for [`RecordMetricsSidecar`].
pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(30);

/// A sidecar which records metrics to a file via EMF.
pub struct RecordMetricsSidecar {
    file_path: String,
    interval: Duration,
}

#[buildstructor::buildstructor]
impl RecordMetricsSidecar {
    /// Build an instance. Any `None` will be replaced with the default value.
    #[builder]
    pub fn new(file_path: Option<String>, interval: Option<Duration>) -> Self {
        Self {
            file_path: file_path.unwrap_or_else(|| DEFAULT_FILE_PATH.to_owned()),
            interval: interval.unwrap_or(DEFAULT_INTERVAL),
        }
    }
}

impl Sidecar for RecordMetricsSidecar {
    fn to_expr(
        &self,
        flow_name: &str,
        _location_key: LocationKey,
        _location_type: LocationType,
        location_name: &str,
        dfir_ident: &syn::Ident,
    ) -> syn::Expr {
        let Self {
            file_path,
            interval,
        } = self;

        let root = get_this_crate();
        let namespace = flow_name.replace(char::is_whitespace, "_");
        let interval: proc_macro2::TokenStream = {
            let secs = interval.as_secs();
            let nanos = interval.subsec_nanos();
            quote!(::std::time::Duration::new(#secs, #nanos))
        };

        parse_quote! {
            #root::telemetry::emf::record_metrics_sidecar(&#dfir_ident, #namespace, #location_name, #file_path, #interval)
        }
    }
}

/// Record both Dfir and Tokio metrics, at the given interval, forever.
#[doc(hidden)]
pub fn record_metrics_sidecar(
    dfir: &Dfir<'_>,
    namespace: &'static str,
    location_name: &'static str,
    file_path: &'static str,
    interval: Duration,
) -> impl 'static + Future<Output = Never> {
    assert!(!namespace.contains(char::is_whitespace));

    let mut dfir_intervals = dfir.metrics_intervals();

    async move {
        // Attempt to create log file parent dir.
        if let Some(parent_dir) = std::path::Path::new(file_path).parent()
            && let Err(e) = tokio::fs::create_dir_all(parent_dir).await
        {
            // TODO(minwgei): use `tracing` once deployments set up tracing logging (setup moved out of stdout)
            eprintln!("Failed to create log file directory for EMF metrics: {}", e);
        }

        // Only attempt to get Tokio runtime within async to be safe.
        let rt_monitor = tokio_metrics::RuntimeMonitor::new(&tokio::runtime::Handle::current());
        let mut rt_intervals = rt_monitor.intervals();

        loop {
            let _ = tokio::time::sleep(interval).await;

            let dfir_metrics = dfir_intervals.take_interval();
            let rt_metrics = rt_intervals.next().unwrap();

            let unwind_result = AssertUnwindSafe(async {
                let timestamp = SystemTime::now();

                let file = tokio::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .append(true)
                    .open(file_path)
                    .await
                    .expect("Failed to open log file for EMF metrics.");
                let mut writer = tokio::io::BufWriter::new(file);

                record_metrics_dfir(
                    namespace,
                    location_name,
                    timestamp,
                    dfir_metrics,
                    &mut writer,
                )
                .await
                .unwrap();

                record_metrics_tokio(namespace, location_name, timestamp, rt_metrics, &mut writer)
                    .await
                    .unwrap();

                writer.shutdown().await.unwrap();
            })
            .catch_unwind()
            .await;

            if let Err(panic_reason) = unwind_result {
                // TODO(minwgei): use `tracing` once deployments set up tracing logging (setup coordination moved out of stdout)
                eprintln!("Panic in metrics sidecar: {panic_reason:?}");
            }
        }
    }
}

/// Records DFIR metrics.
async fn record_metrics_dfir<W>(
    namespace: &str,
    location_name: &str,
    timestamp: SystemTime,
    metrics: DfirMetrics,
    writer: &mut W,
) -> Result<(), std::io::Error>
where
    W: AsyncWrite + Unpin,
{
    let ts_millis = timestamp
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    // Handoffs
    for (hoff_id, hoff_metrics) in metrics.handoffs.iter() {
        let emf = json!({
            "_aws": {
                "Timestamp": ts_millis,
                "CloudWatchMetrics": [
                    {
                        "Namespace": namespace,
                        "Dimensions": [["LocationName"], ["LocationName", "HandoffId"]],
                        "Metrics": [
                            {"Name": "CurrItemsCount", "Unit": Unit::Count},
                            {"Name": "TotalItemsCount", "Unit": Unit::Count},
                        ]
                    }
                ]
            },
            "LocationName": location_name,
            "HandoffId": hoff_id.to_string(),
            "CurrItemsCount": hoff_metrics.curr_items_count(),
            "TotalItemsCount": hoff_metrics.total_items_count(),
        })
        .to_string();
        writer.write_all(emf.as_bytes()).await?;
        writer.write_u8(b'\n').await?;
    }

    // Subgraphs
    for (sg_id, sg_metrics) in metrics.subgraphs.iter() {
        let emf = json!({
            "_aws": {
                "Timestamp": ts_millis,
                "CloudWatchMetrics": [
                    {
                        "Namespace": namespace,
                        "Dimensions": [["LocationName"], ["LocationName", "SubgraphId"]],
                        "Metrics": [
                            {"Name": "TotalRunCount", "Unit": Unit::Count},
                            {"Name": "TotalPollDuration", "Unit": Unit::Microseconds},
                            {"Name": "TotalPollCount", "Unit": Unit::Count},
                            {"Name": "TotalIdleDuration", "Unit": Unit::Microseconds},
                            {"Name": "TotalIdleCount", "Unit": Unit::Count},
                        ]
                    }
                ]
            },
            "LocationName": location_name,
            "SubgraphId": sg_id.to_string(),
            "TotalRunCount": sg_metrics.total_run_count(),
            "TotalPollDuration": sg_metrics.total_poll_duration().as_micros(),
            "TotalPollCount": sg_metrics.total_poll_count(),
            "TotalIdleDuration": sg_metrics.total_idle_duration().as_micros(),
            "TotalIdleCount": sg_metrics.total_idle_count(),
        })
        .to_string();
        writer.write_all(emf.as_bytes()).await?;
        writer.write_u8(b'\n').await?;
    }

    Ok(())
}

/// Records tokio runtime metrics.
async fn record_metrics_tokio<W>(
    namespace: &str,
    location_name: &str,
    timestamp: SystemTime,
    rt_metrics: RuntimeMetrics,
    writer: &mut W,
) -> Result<(), std::io::Error>
where
    W: AsyncWrite + Unpin,
{
    let ts_millis = timestamp
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    // Tokio RuntimeMetrics
    let emf = json!({
        "_aws": {
            "Timestamp": ts_millis,
            "CloudWatchMetrics": [
                {
                    "Namespace": namespace,
                    "Dimensions": [["LocationName"]],
                    "Metrics": [
                        // {"Name": "LiveTasksCount", "Unit": Unit::Count}, // https://github.com/tokio-rs/tokio-metrics/pull/108
                        {"Name": "TotalBusyDuration", "Unit": Unit::Microseconds},
                        {"Name": "GlobalQueueDepth", "Unit": Unit::Count},
                    ]
                }
            ]
        },
        "LocationName": location_name,
        // "LiveTasksCount": rt_metrics.live_tasks_count, // https://github.com/tokio-rs/tokio-metrics/pull/108
        "TotalBusyDuration": rt_metrics.total_busy_duration.as_micros(),
        "GlobalQueueDepth": rt_metrics.global_queue_depth,
        // The rest of the tokio runtime metrics are `cfg(tokio_unstable)`
    })
    .to_string();
    writer.write_all(emf.as_bytes()).await?;
    writer.write_u8(b'\n').await?;

    Ok(())
}

/// AWS CloudWatch EMF units.
///
/// <https://docs.aws.amazon.com/AmazonCloudWatch/latest/APIReference/API_MetricDatum.html#ACW-Type-MetricDatum-Unit>
#[expect(missing_docs, reason = "self-explanatory")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Unit {
    /// None
    #[default]
    None,
    Seconds,
    Microseconds,
    Milliseconds,
    Bytes,
    Kilobytes,
    Megabytes,
    Gigabytes,
    Terabytes,
    Bits,
    Kilobits,
    Megabits,
    Gigabits,
    Terabits,
    Percent,
    Count,
    #[serde(rename = "Bytes/Second")]
    BytesPerSecond,
    #[serde(rename = "Kilobytes/Second")]
    KilobytesPerSecond,
    #[serde(rename = "Megabytes/Second")]
    MegabytesPerSecond,
    #[serde(rename = "Gigabytes/Second")]
    GigabytesPerSecond,
    #[serde(rename = "Terabytes/Second")]
    TerabytesPerSecond,
    #[serde(rename = "Bits/Second")]
    BitsPerSecond,
    #[serde(rename = "Kilobits/Second")]
    KilobitsPerSecond,
    #[serde(rename = "Megabits/Second")]
    MegabitsPerSecond,
    #[serde(rename = "Gigabits/Second")]
    GigabitsPerSecond,
    #[serde(rename = "Terabits/Second")]
    TerabitsPerSecond,
    #[serde(rename = "Count/Second")]
    CountPerSecond,
}
