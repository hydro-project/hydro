#[hydroflow::main]
async fn main() {
    let _ = hydroflow::util::deploy::init::<()>().await;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
