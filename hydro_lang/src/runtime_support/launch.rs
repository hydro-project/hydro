use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(feature = "runtime_measure")]
use std::panic::AssertUnwindSafe;

#[cfg(feature = "runtime_measure")]
use dfir_rs::futures::FutureExt;
use dfir_rs::scheduled::graph::Dfir;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
pub use hydro_deploy_integration::*;
#[cfg(feature = "runtime_measure")]
#[cfg(target_os = "linux")]
use procfs::WithCurrentSystemInfo;
use serde::de::DeserializeOwned;

#[cfg(not(feature = "runtime_measure"))]
pub async fn run(flow: Dfir<'_>) {
    launch_flow(flow).await;
}

pub async fn run_containerized(flow: Dfir<'_>) {
    launch_flow_containerized(flow).await;
}

#[cfg(feature = "runtime_measure")]
pub async fn run(flow: Dfir<'_>) {
    // Make sure to print CPU even if we crash
    let res = AssertUnwindSafe(launch_flow(flow)).catch_unwind().await;

    #[cfg(target_os = "linux")]
    {
        let me = procfs::process::Process::myself().unwrap();
        let stat = me.stat().unwrap();
        let sysinfo = procfs::current_system_info();

        let start_time = stat.starttime().get().unwrap();
        let curr_time = chrono::Local::now();
        let elapsed_time = curr_time - start_time;

        let seconds_spent = (stat.utime + stat.stime) as f32 / sysinfo.ticks_per_second() as f32;
        let run_time = chrono::Duration::milliseconds((seconds_spent * 1000.0) as i64);

        let percent_cpu_use =
            run_time.num_milliseconds() as f32 / elapsed_time.num_milliseconds() as f32;
        let user_time = chrono::Duration::milliseconds(
            (stat.utime as f32 / sysinfo.ticks_per_second() as f32 * 1000.0) as i64,
        );
        let user_cpu_use =
            user_time.num_milliseconds() as f32 / elapsed_time.num_milliseconds() as f32;
        let system_time = chrono::Duration::milliseconds(
            (stat.stime as f32 / sysinfo.ticks_per_second() as f32 * 1000.0) as i64,
        );
        let system_cpu_use =
            system_time.num_milliseconds() as f32 / elapsed_time.num_milliseconds() as f32;
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
        // TODO(shadaj): can enable on next sysinfo release
        // use sysinfo::{Pid, System};
        // let system = System::new_all();
        // let process = system.process(Pid::from_u32(std::process::id())).unwrap();
        // let run_time = process.run_time() * 1000;
        // let cpu_time = process.accumulated_cpu_time();
        // let user_cpu_use = cpu_time.user() as f32 / run_time as f32;
        let user_cpu_use = 100.0;

        println!(
            "{} Total {:.4}%, User {:.4}%, System {:.4}%",
            option_env!("HYDRO_RUNTIME_MEASURE_CPU_PREFIX").unwrap_or("CPU:"),
            user_cpu_use,
            user_cpu_use,
            0.0
        );
    }

    res.unwrap();
}



pub async fn launch_flow(mut flow: Dfir<'_>) {
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

    let local_set = tokio::task::LocalSet::new();
    let flow = local_set.run_until(flow.run());

    tokio::select! {
        _ = stop.1 => {},
        _ = flow => {}
    }
}

pub async fn launch_flow_containerized(mut flow: Dfir<'_>) {
    let local_set = tokio::task::LocalSet::new();
    local_set.run_until(flow.run()).await;
}

pub async fn init_no_ack_start<T: DeserializeOwned + Default>() -> DeployPorts<T> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let trimmed = input.trim();

    let bind_config = serde_json::from_str::<InitConfig>(trimmed).unwrap();

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

    let all_connected = client_conns
        .into_iter()
        .chain(server_conns.into_iter())
        .collect();

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
