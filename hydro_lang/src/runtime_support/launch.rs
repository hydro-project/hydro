use dfir_rs::scheduled::graph::Dfir;
use tokio::runtime::Runtime;

#[cfg_attr(feature = "runtime_io-uring", allow(unused))]
pub fn launch(tokio_rt: Runtime, flow: Dfir<'_>) {
    #[cfg(not(feature = "runtime_io-uring"))]
    tokio_rt.block_on(async {
        println!("ack start");
        super::resource_measurement::run(flow).await;
    });

    #[cfg(feature = "runtime_io-uring")]
    {
        tokio_uring::builder().entries(4096).start(async move {
            println!("ack start");
            super::resource_measurement::run(flow).await;
        })
    }
}
