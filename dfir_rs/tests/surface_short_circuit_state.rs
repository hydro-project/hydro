//! Tests that short-circuiting operators do not break stateful operators.
//! See https://github.com/hydro-project/hydro/issues/2334
use std::time::Duration;

use dfir_rs::util::collect_ready_async;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test(dfir)]
async fn test_resolve_futures_cross_singleton() {
    let (singleton_send, singleton_recv) = dfir_rs::util::unbounded_channel::<()>();
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<u64>();
    let (output_send, mut output_recv) = dfir_rs::util::unbounded_channel::<u64>();

    let mut df = dfir_syntax! {
        source_stream(items_recv)
            -> map(|millis| {
                println!("mapping {}", millis);
                async move {
                    println!("running {}", millis);
                    tokio::time::sleep(Duration::from_millis(millis)).await;
                    millis
                }}
            )
            -> resolve_futures()
            -> [input]cs;
        source_stream(singleton_recv) -> [single]cs;
        cs = cross_singleton() -> map(|(millis, ())| millis) -> for_each(|millis| output_send.send(millis).unwrap());
    };

    assert_graphvis_snapshots!(df);

    items_send.send(500).unwrap();
    items_send.send(510).unwrap();
    items_send.send(520).unwrap();
    df.run_tick().await;

    println!("tick over");

    tokio::time::sleep(Duration::from_millis(1000)).await;

    println!("tick start");

    singleton_send.send(()).unwrap();
    df.run_tick().await;

    let out = collect_ready_async::<Vec<_>, _>(&mut output_recv).await;
    assert_eq!(&[500, 510, 520], &*out);
}

#[multiplatform_test(dfir)]
async fn test_zip_cross_singleton() {
    let (a_send, a_recv) = dfir_rs::util::unbounded_channel::<&'static str>();
    let (b_send, b_recv) = dfir_rs::util::unbounded_channel::<u64>();
    let (singleton_send, singleton_recv) = dfir_rs::util::unbounded_channel::<()>();
    let (output_send, mut output_recv) = dfir_rs::util::unbounded_channel::<(&'static str, u64)>();

    let mut df = dfir_syntax! {
        source_stream(a_recv) -> [0]z;
        source_stream(b_recv) -> [1]z;

        z = zip::<'static, 'static>() -> [input]cs;
        source_stream(singleton_recv) -> [single]cs;
        cs = cross_singleton() -> map(|(millis, ())| millis) -> for_each(|millis| output_send.send(millis).unwrap());
    };

    a_send.send("foo").unwrap();
    a_send.send("bar").unwrap();
    a_send.send("baz").unwrap();
    a_send.send("ding").unwrap();
    b_send.send(5).unwrap();
    df.run_tick().await;

    singleton_send.send(()).unwrap();
    a_send.send("doom").unwrap();
    b_send.send(6).unwrap();
    b_send.send(7).unwrap();
    df.run_tick().await;

    let out = collect_ready_async::<Vec<_>, _>(&mut output_recv).await;
    assert_eq!([("bar", 6), ("baz", 7)], &*out);
}
