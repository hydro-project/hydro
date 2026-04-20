use multiplatform_test::multiplatform_test;

#[multiplatform_test]
fn test_surface_flows_1() {
    let mut df = dfir_rs::dfir_syntax_inline! {
        my_tee = source_iter(vec!["Hello", "world"]) -> tee();
        my_tee[0] -> map(|x| x.to_uppercase()) -> [0]my_union;
        my_tee[1] -> map(|x| x.to_lowercase()) -> [1]my_union;
        my_union = union() -> for_each(|x| println!("{}", x));
    };
    df.run_available_sync();
}

#[dfir_rs::test]
async fn test_source_interval() {
    use dfir_rs::dfir_syntax_inline;
    use web_time::{Duration, Instant};

    let mut hf = dfir_syntax_inline! {
        source_interval(Duration::from_secs(1))
            -> map(|_| { Instant::now() } )
            -> for_each(|time| println!("This runs every second: {:?}", time));
    };

    // Will print 4 times (fencepost counting).
    tokio::time::timeout(Duration::from_secs_f32(3.5), hf.run())
        .await
        .expect_err("Expected time out");
}
