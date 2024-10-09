use hydroflow::util::deploy::{ConnectedDirect, ConnectedSource};
use hydroflow::util::deserialize_from_bytes;
use hydroflow_datalog::datalog;

#[hydroflow::main]
async fn main() {
    let ports = hydroflow::util::deploy::init::<()>().await;
    let broadcast_recv = ports
        .port("broadcast")
        .connect::<ConnectedDirect>()
        .await
        .into_source();

    let df = datalog!(
        r#"
        .async broadcast `null::<(String,)>()` `source_stream(broadcast_recv) -> map(|x| deserialize_from_bytes::<(String,)>(x.unwrap()).unwrap())`
        .output stdout `for_each(|tup| println!("echo {:?}", tup))`

        stdout(x) :- broadcast(x)
    "#
    );

    hydroflow::util::deploy::launch_flow(df).await;
}
