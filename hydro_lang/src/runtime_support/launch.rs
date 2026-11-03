use std::cell::RefCell;
use std::collections::HashMap;

#[cfg(feature = "runtime_measure")]
use dfir_rs::futures::FutureExt;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
pub use hydro_deploy_integration::*;
#[cfg(feature = "runtime_measure")]
#[cfg(target_os = "linux")]
use procfs::WithCurrentSystemInfo;
use serde::de::DeserializeOwned;

#[cfg(not(feature = "runtime_measure"))]
pub async fn run_stdin_commands(flow: impl Future) {
    launch_flow_stdin_commands(flow).await;
}

/// Fractions of wall-clock `runtime` spent on CPU.
///
/// Returns `(user, system, total)` where `total = user + system`. Values are
/// fractions of one core, not percentages, and may be greater than 1.
#[cfg(feature = "runtime_measure")]
fn cpu_use_fractions(
    user_cpu_time: std::time::Duration,
    system_cpu_time: std::time::Duration,
    runtime: std::time::Duration,
) -> (f32, f32, f32) {
    let runtime_ms = runtime.as_millis() as f32;
    // A zero runtime (for example sysinfo's whole-second counter on a short
    // process) cannot be turned into a fraction.
    if runtime_ms == 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let user_fraction = user_cpu_time.as_millis() as f32 / runtime_ms;
    let system_fraction = system_cpu_time.as_millis() as f32 / runtime_ms;
    (
        user_fraction,
        system_fraction,
        user_fraction + system_fraction,
    )
}

#[cfg(feature = "runtime_measure")]
pub async fn run_stdin_commands(flow: impl Future) {
    // Make sure to print CPU even if we crash
    let res = std::panic::AssertUnwindSafe(launch_flow_stdin_commands(flow))
        .catch_unwind()
        .await;

    #[cfg(target_os = "linux")]
    {
        let me = procfs::process::Process::myself().unwrap();
        let stat = me.stat().unwrap();
        let sysinfo = procfs::current_system_info();

        let start_time = stat.starttime().get().unwrap();
        let curr_time = chrono::Local::now();
        let elapsed_time = curr_time - start_time;
        let ticks_per_second = sysinfo.ticks_per_second() as f32;

        // Truncate each counter to milliseconds before dividing, matching the
        // previous procfs report.
        let user_cpu_time = std::time::Duration::from_millis(
            (stat.utime as f32 / ticks_per_second * 1000.0) as u64,
        );
        let system_cpu_time = std::time::Duration::from_millis(
            (stat.stime as f32 / ticks_per_second * 1000.0) as u64,
        );
        let runtime = std::time::Duration::from_millis(elapsed_time.num_milliseconds() as u64);
        let (user_cpu_use, system_cpu_use, percent_cpu_use) =
            cpu_use_fractions(user_cpu_time, system_cpu_time, runtime);
        println!(
            "{} Total {:.4}%, User {:.4}%, System {:.4}%",
            option_env!("HYDRO_RUNTIME_MEASURE_CPU_PREFIX").unwrap_or("CPU:"),
            percent_cpu_use,
            user_cpu_use,
            system_cpu_use
        );
    }

    #[cfg(not(target_os = "linux"))]
    {
        use std::time::Duration;

        use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

        let system = System::new_with_specifics(
            RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing().with_cpu()),
        )
        .expect("failed to read process CPU counters");
        let process = system
            .process(Pid::from_u32(std::process::id()))
            .expect("current process missing from sysinfo");
        // `accumulated_cpu_time` is user+system CPU time in milliseconds.
        // sysinfo 0.40 does not expose the two modes separately.
        let (user_cpu_use, system_cpu_use, total_cpu_use) = cpu_use_fractions(
            Duration::from_millis(process.accumulated_cpu_time()),
            Duration::ZERO,
            Duration::from_secs(process.run_time()),
        );

        println!(
            "{} Total {:.4}%, User {:.4}%, System {:.4}%",
            option_env!("HYDRO_RUNTIME_MEASURE_CPU_PREFIX").unwrap_or("CPU:"),
            total_cpu_use,
            user_cpu_use,
            system_cpu_use
        );
    }

    res.unwrap();
}

pub async fn launch_flow_stdin_commands(flow: impl Future) {
    // TODO(mingwei): convert to use CancellationToken at some point
    // Not trivial: https://github.com/hydro-project/hydro/pull/2495/changes#r2733428502
    let stop = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(|| {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        if line.starts_with("stop") {
            stop.0.send(()).unwrap();
        } else {
            eprintln!("Unexpected stdin input: {:?}", line);
        }
    });

    tokio::select! {
        _ = stop.1 => {},
        _ = flow => {}
    }
}

pub async fn init_no_ack_start<T: DeserializeOwned + Default>() -> DeployPorts<T> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let trimmed = input.trim();

    let bind_config = serde_json::from_str::<InitConfig<'_>>(trimmed).unwrap();

    // config telling other services how to connect to me
    let mut bind_results: HashMap<String, ServerPort> = HashMap::new();
    let mut binds = HashMap::new();
    for (name, config) in bind_config.0 {
        let bound = config.bind().await;
        bind_results.insert(name.clone(), bound.server_port());
        binds.insert(name.clone(), bound);
    }

    let bind_serialized = serde_json::to_string(&bind_results).unwrap();
    println!("ready: {bind_serialized}");

    let mut start_buf = String::new();
    std::io::stdin().read_line(&mut start_buf).unwrap();
    let connection_defns = if start_buf.starts_with("start: ") {
        serde_json::from_str::<HashMap<String, ServerPort>>(
            start_buf.trim_start_matches("start: ").trim(),
        )
        .unwrap()
    } else {
        panic!("expected start");
    };

    let (client_conns, server_conns) = futures::join!(
        connection_defns
            .into_iter()
            .map(|(name, defn)| async move { (name, Connection::AsClient(defn.connect().await)) })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>(),
        binds
            .into_iter()
            .map(
                |(name, defn)| async move { (name, Connection::AsServer(accept_bound(defn).await)) }
            )
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
    );

    let all_connected = client_conns.into_iter().chain(server_conns).collect();

    DeployPorts {
        ports: RefCell::new(all_connected),
        meta: bind_config
            .1
            .map(|b| serde_json::from_str(&b).unwrap())
            .unwrap_or_default(),
    }
}

pub async fn init<T: DeserializeOwned + Default>() -> DeployPorts<T> {
    let ret = init_no_ack_start::<T>().await;

    println!("ack start");

    ret
}

#[cfg(all(test, feature = "runtime_measure"))]
mod tests {
    use std::time::Duration;

    #[test]
    fn cpu_use_fractions_from_injected_durations() {
        // user 250ms, system 250ms, runtime 1000ms -> 0.25, 0.25, 0.50
        let (user, system, total) = super::cpu_use_fractions(
            Duration::from_millis(250),
            Duration::from_millis(250),
            Duration::from_millis(1000),
        );
        assert_eq!(user, 0.25);
        assert_eq!(system, 0.25);
        assert_eq!(total, 0.50);
    }
}
