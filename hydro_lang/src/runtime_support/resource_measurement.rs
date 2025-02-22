#[cfg(feature = "runtime_measure")]
use std::panic::AssertUnwindSafe;

#[cfg(feature = "runtime_measure")]
use dfir_rs::futures::FutureExt;
use dfir_rs::scheduled::graph::Dfir;
#[cfg(feature = "runtime_measure")]
#[cfg(target_os = "linux")]
use procfs::WithCurrentSystemInfo;

pub const CPU_USAGE_PREFIX: &str = "CPU:";
// Should remain consistent with dfir_lang/src/graph/ops/_counter.rs
pub const COUNTER_PREFIX: &str = "_counter";

#[cfg(not(feature = "runtime_measure"))]
pub async fn run(flow: Dfir<'_>) {
    dfir_rs::util::deploy::launch_flow(flow).await;
}

#[cfg(feature = "runtime_measure")]
pub async fn run(flow: Dfir<'_>) {
    // Make sure to print CPU even if we crash
    let res = AssertUnwindSafe(dfir_rs::util::deploy::launch_flow(flow))
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
            CPU_USAGE_PREFIX, percent_cpu_use, user_cpu_use, system_cpu_use
        );
    }

    res.unwrap();
}
