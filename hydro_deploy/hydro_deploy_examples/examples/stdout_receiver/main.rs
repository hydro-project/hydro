use dfir_rs::dfir_syntax;
use dfir_rs::util::deploy::{ConnectedDirect, ConnectedSource};

#[dfir_rs::main]
async fn main() {
    let ports = dfir_rs::util::deploy::init::<()>().await;
    let echo_recv = ports
        .port("echo")
        .connect::<ConnectedDirect>()
        .await
        .into_source();

    let df = dfir_syntax! {
        source_stream(echo_recv) ->
            map(|x| String::from_utf8(x.unwrap().to_vec()).unwrap()) ->
            for_each(|x| println!("echo {:?}", x));
    };

    dfir_rs::util::deploy::launch_flow(df).await;
}
