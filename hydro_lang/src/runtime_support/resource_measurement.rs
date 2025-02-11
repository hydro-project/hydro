
use dfir_rs::scheduled::graph::Dfir;

pub async fn run(flow: Dfir<'_>) {
    dfir_rs::util::deploy::launch_flow(flow).await;
}

pub async fn run_with_measurement(flow: Dfir<'_>) {
    run(flow).await;

    #[cfg(target_os = "linux")]
    let me = procfs::process::Process::myself().unwrap();
}