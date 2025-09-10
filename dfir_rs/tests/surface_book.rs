use dfir_rs::assert_graphvis_snapshots;
use multiplatform_test::multiplatform_test;
use tokio::time::Instant;

#[multiplatform_test]
fn test_surface_flows_1() {
    let mut df = dfir_rs::dfir_syntax! {
        my_tee = source_iter(vec!["Hello", "world"]) -> tee();
        my_tee[0] -> map(|x| x.to_uppercase()) -> [0]my_union;
        my_tee[1] -> map(|x| x.to_lowercase()) -> [1]my_union;
        my_union = union() -> for_each(|x| println!("{}", x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();
}

#[dfir_rs::test]
async fn test_source_interval() {
    use std::time::Duration;

    use dfir_rs::dfir_syntax;

    let mut hf = dfir_syntax! {
        source_interval(Duration::from_secs(1))
            -> map(|_| { Instant::now() } )
            -> for_each(|time| println!("This runs every second: {:?}", time));
    };

    // Will print 4 times (fencepost counting).
    tokio::time::timeout(Duration::from_secs_f32(3.5), hf.run())
        .await
        .expect_err("Expected time out");
}
