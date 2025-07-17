#[dfir_rs::main]
async fn main() {
    let _ = dfir_rs::util::deploy::init::<()>().await;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
