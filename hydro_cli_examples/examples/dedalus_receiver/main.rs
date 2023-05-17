use hydroflow::util::cli::{ConnectedBidi, ConnectedSource};
use hydroflow::util::deserialize_from_bytes;
use hydroflow_datalog::datalog;

#[tokio::main]
async fn main() {
    let mut ports = hydroflow::util::cli::init().await;
    let broadcast_recv = ports
        .port("broadcast")
        .connect::<ConnectedBidi>()
        .await
        .into_source();

    let df = datalog!(
        r#"
        .async broadcast `null::<(String,)>()` `source_stream(broadcast_recv) -> map(|x| deserialize_from_bytes::<(String,)>(x.unwrap()).unwrap())`
        .output stdout `for_each(|tup| println!("echo {:?}", tup))`

        stdout(x) :- broadcast(x)
    "#
    );

    hydroflow::util::cli::launch_flow(df).await;
}
