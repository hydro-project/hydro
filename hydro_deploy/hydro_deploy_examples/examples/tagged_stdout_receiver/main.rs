use dfir_rs::dfir_syntax;
use dfir_rs::util::deploy::{ConnectedDirect, ConnectedSource, ConnectedTagged};

#[dfir_rs::main]
async fn main() {
    let ports = dfir_rs::util::deploy::init::<()>().await;
    let echo_recv = ports
        .port("echo")
        .connect::<ConnectedTagged<u32, ConnectedDirect>>()
        .into_source();

    let df = dfir_syntax! {
        source_stream(echo_recv) ->
            map(|x| {
                let x = x.unwrap();
                (x.0, String::from_utf8(x.1.to_vec()).unwrap())
            }) ->
            for_each(|x| println!("echo {:?}", x));
    };

    dfir_rs::util::deploy::launch_flow(df).await;
}
