use dfir_datalog::datalog;
use dfir_rs::util::deploy::{ConnectedDirect, ConnectedSource};
use dfir_rs::util::deserialize_from_bytes;

#[dfir_rs::main]
async fn main() {
    let ports = dfir_rs::util::deploy::init::<()>().await;
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

    dfir_rs::util::deploy::launch_flow(df).await;
}
