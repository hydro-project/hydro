use std::cell::RefCell;
use std::rc::Rc;

use dfir_rs::dfir_expect_warnings;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::collect_ready;

#[test]
fn test_degenerate_union() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_expect_warnings! {
        {
            source_iter([1, 2, 3]) -> union() -> for_each(|x| result_send.send(x).unwrap());
        },
        ("`union` should have at least 2 input(s), actually has 1.", 2:38),
    };
    df.run_available_sync();

    assert_eq!(&[1, 2, 3], &*collect_ready::<Vec<_>, _>(&mut result_recv));
}

#[test]
fn test_empty_union() {
    let mut df = dfir_expect_warnings! {
        {
            union() -> for_each(|x: usize| println!("{}", x));
        },
        ("`union` should have at least 2 input(s), actually has 0.", 2:12),
    };
    df.run_available_sync();
}

#[test]
fn test_degenerate_tee() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_expect_warnings! {
        {
            source_iter([1, 2, 3]) -> tee() -> for_each(|x| result_send.send(x).unwrap());
        },
        ("`tee` should have at least 2 output(s), actually has 1.", 2:38)
    };
    df.run_available_sync();

    assert_eq!(&[1, 2, 3], &*collect_ready::<Vec<_>, _>(&mut result_recv));
}

#[test]
fn test_empty_tee() {
    let output = <Rc<RefCell<Vec<usize>>>>::default();
    let output_inner = Rc::clone(&output);

    let mut df = dfir_expect_warnings! {
        {
            source_iter([1, 2, 3]) -> inspect(|&x| output_inner.borrow_mut().push(x)) -> tee();
        },
        ("`tee` should have at least 2 output(s), actually has 0.", 2:89),
    };
    df.run_available_sync();

    assert_eq!(&[1, 2, 3], &**output.borrow());
}

// Mainly checking subgraph partitioning pull-push handling.
// But in a different edge order.
#[test]
pub fn test_warped_diamond() {
    let mut df = dfir_expect_warnings! {
        {
            // active nodes
            nodes = union();

            // stream of nodes into the system
            init = join() -> for_each(|(n, (a, b))| {
                println!("DEBUG ({:?}, ({:?}, {:?}))", n, a, b);
            });
            new_node = source_iter([1, 2, 3]) -> tee();
            // add self
            new_node[0] -> map(|n| (n, 'a')) -> [0]nodes;
            // join peers against active nodes
            nodes -> [0]init;
            new_node[1] -> map(|n| (n, 'b')) -> [1]init;
        },
        ("`union` should have at least 2 input(s), actually has 1.", 3:20),
    };
    df.run_available_sync();
}

#[test]
pub fn test_warped_diamond_2() {
    let mut hf: Dfir = dfir_expect_warnings! {
        {
            // active nodes
            nodes = union();

            // stream of nodes into the system
            init = join() -> for_each(|(n, (a, b))| {
                println!("DEBUG ({:?}, ({:?}, {:?}))", n, a, b);
            });
            new_node = source_iter([1, 2, 3]) -> tee();
            // add self
            new_node[0] -> map(|n| (n, 'a')) -> [0]nodes;
            // join peers against active nodes
            nodes -> [0]init;
            new_node[1] -> map(|n| (n, 'b')) -> [1]init;

            ntwk = source_iter([4, 5, 6]) -> tee();
        },
        ("`union` should have at least 2 input(s), actually has 1.", 3:20),
        ("`tee` should have at least 2 output(s), actually has 0.", 16:45),
    };
    hf.run_available_sync();
}
