//! Isolated test for persist_mut push-side borrow conflict.
use dfir_rs::dfir_syntax;
use dfir_rs::util::Persistence::*;
use dfir_rs::util::collect_ready;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_persist_mut_push() {
    let (pull_tx, mut pull_rx) = dfir_rs::util::unbounded_channel::<usize>();
    let (push_tx, mut push_rx) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        my_tee = source_iter([Persist(1), Persist(2), Persist(3), Persist(4), Delete(2)])
            -> persist_mut::<'mutable>()
            -> tee();

        my_tee
            -> for_each(|v| pull_tx.send(v).unwrap());

        my_tee
            -> flat_map(|x| if x == 3 {vec![Persist(x), Delete(x)]} else {vec![Persist(x)]})
            -> persist_mut::<'mutable>()
            -> for_each(|v| push_tx.send(v).unwrap());
    };

    df.run_available_sync();
}
