use std::cell::RefCell;
use std::rc::Rc;

use dfir_rs::dfir_syntax_inline;

/// Test hypothesis: defer_tick_lazy in a cycle delays by 2 ticks instead of 1
/// (off-by-one in stratification).
#[dfir_rs::test]
async fn test_defer_cycle_extra_tick() {
    let output = Rc::new(RefCell::new(Vec::<usize>::new()));
    let output_inner = Rc::clone(&output);

    let mut df = dfir_syntax_inline! {
        a = union() -> tee();
        source_iter([1_usize, 3]) -> [0]a;
        a[0] -> defer_tick_lazy() -> map(|x: usize| 2 * x) -> [1]a;
        a[1] -> for_each(|x: usize| output_inner.borrow_mut().push(x));
    };

    df.run_tick().await;
    let r0 = output.take();
    eprintln!("tick 0: {:?}", r0);

    df.run_tick().await;
    let r1 = output.take();
    eprintln!("tick 1: {:?}", r1);

    df.run_tick().await;
    let r2 = output.take();
    eprintln!("tick 2: {:?}", r2);

    df.run_tick().await;
    let r3 = output.take();
    eprintln!("tick 3: {:?}", r3);

    // Expected if defer delays by 1 tick:
    //   tick 0: [1, 3]
    //   tick 1: [2, 6]
    //   tick 2: [4, 12]
    //   tick 3: [8, 24]
    //
    // If defer delays by 2 ticks (off-by-one in stratification):
    //   tick 0: [1, 3]
    //   tick 1: []
    //   tick 2: [2, 6]
    //   tick 3: []
}
