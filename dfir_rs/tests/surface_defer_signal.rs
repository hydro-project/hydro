use dfir_rs::util::collect_ready_async;
use dfir_rs::dfir_syntax_inline;

#[dfir_rs::test]
pub async fn test_basic_2() {
    let (signal_tx, signal_rx) = dfir_rs::util::unbounded_channel::<()>();
    let (egress_tx, mut egress_rx) = dfir_rs::util::unbounded_channel();

    let mut df = dfir_syntax_inline! {
        gate = defer_signal();
        source_iter([1, 2, 3]) -> [input]gate;
        source_stream(signal_rx) -> [signal]gate;

        gate -> for_each(|x| egress_tx.send(x).unwrap());
    };

    df.run_available().await;
    let out: Vec<_> = collect_ready_async(&mut egress_rx).await;
    assert_eq!(out, [0; 0]);

    signal_tx.send(()).unwrap();
    df.run_available().await;

    let out: Vec<_> = collect_ready_async(&mut egress_rx).await;
    assert_eq!(out, vec![1, 2, 3]);
}
