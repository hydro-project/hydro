
use dfir_rs::scheduled::graph::Dfir;
use sysinfo::{CpuRefreshKind, RefreshKind, System};

const MEASUREMENT_OUT_FILE: &str = "cpu_usage.txt";

async fn run(flow: Dfir<'_>) {
    dfir_rs::util::deploy::launch_flow(flow).await;
}

async fn run_with_measurement(flow: Dfir<'_>) {
    let mut s = System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing().with_cpu_usage()));
    
    run(flow).await;

    // Record CPU usage
    s.refresh_cpu_all();
    let mut out_string = String::new();
    for cpu in s.cpus() {
        out_string.push_str(format!("CPU {:?}: {}%", cpu.name(), cpu.cpu_usage()).as_str());
    }
    std::fs::write(MEASUREMENT_OUT_FILE, out_string).unwrap();
}