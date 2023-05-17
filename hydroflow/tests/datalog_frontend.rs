use hydroflow::datalog;
use hydroflow::util::collect_ready;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_minimal() {
    let (in_send, input) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    in_send.send((1, 2)).unwrap();

    let mut flow = datalog!(
        r#"
        .input input `source_stream(input)`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(y, x) :- input(x, y).
        "#
    );

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[(2, 1)]);
}

#[multiplatform_test]
pub fn test_minimal_static() {
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut flow = datalog!(
        r#"
        .static input `vec![(1, 2), (3, 4)]`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(y, x) :- input(x, y).
        "#
    );

    flow.run_tick();
    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut out_recv),
        &[(2, 1), (4, 3)]
    );
    flow.run_tick();
    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut out_recv),
        &[(2, 1), (4, 3)]
    );
}

#[multiplatform_test]
pub fn test_duplicated_facts() {
    let (in_send, input) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    in_send.send((1, 2)).unwrap();

    let mut flow = datalog!(
        r#"
        .input input `source_stream(input)`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(y, x) :- input(x, y).
        out(y, x) :- input(x, y).
        "#
    );

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[(2, 1)]);
}

#[multiplatform_test]
pub fn test_join_with_self() {
    let (in_send, input) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    in_send.send((1, 2)).unwrap();
    in_send.send((2, 1)).unwrap();
    in_send.send((1, 3)).unwrap();

    let mut flow = datalog!(
        r#"
        .input input `source_stream(input)`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(x, y) :- input(x, y), input(y, x).
        "#
    );

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut out_recv),
        &[(2, 1), (1, 2)]
    );
}

#[multiplatform_test]
pub fn test_wildcard_fields() {
    let (in_send, input) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize,)>();

    let mut flow = datalog!(
        r#"
        .input input `source_stream(input)`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(x) :- input(x, _), input(_, x).
        "#
    );

    in_send.send((1, 2)).unwrap();
    in_send.send((3, 1)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[(1,)]);
}

#[multiplatform_test]
pub fn test_multi_use_intermediate() {
    let (in_send, input) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    in_send.send((1, 2)).unwrap();
    in_send.send((2, 1)).unwrap();
    in_send.send((1, 3)).unwrap();

    let mut flow = datalog!(
        r#"
        .input input `source_stream(input)`
        .output out `for_each(|v| out.send(v).unwrap())`

        in_dup(x, y) :- input(x, y).
        out(x, y) :- in_dup(x, y), in_dup(y, x).
        "#
    );

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut out_recv),
        &[(2, 1), (1, 2)]
    );
}

#[multiplatform_test]
pub fn test_join_with_other() {
    let (in1_send, in1) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (in2_send, in2) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut flow = datalog!(
        r#"
        .input in1 `source_stream(in1)`
        .input in2 `source_stream(in2)`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(x, y) :- in1(x, y), in2(y, x).
        "#
    );

    in1_send.send((1, 2)).unwrap();
    in1_send.send((1, 3)).unwrap();
    in2_send.send((2, 1)).unwrap();
    in2_send.send((4, 1)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[(1, 2)]);

    in1_send.send((1, 3)).unwrap();
    in1_send.send((1, 4)).unwrap();
    in2_send.send((3, 1)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[(1, 3)]);
}

#[multiplatform_test]
pub fn test_multiple_contributors() {
    let (in1_send, in1) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (in2_send, in2) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    in1_send.send((1, 2)).unwrap();
    in2_send.send((3, 1)).unwrap();

    let mut flow = datalog!(
        r#"
        .input in1 `source_stream(in1)`
        .input in2 `source_stream(in2)`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(x, y) :- in1(x, y).
        out(x, y) :- in2(y, x).
        "#
    );

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut out_recv),
        &[(1, 2), (1, 3)]
    );
}

#[multiplatform_test]
pub fn test_transitive_closure() {
    let (edges_send, edges) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (seed_reachable_send, seed_reachable) = hydroflow::util::unbounded_channel::<(usize,)>();
    let (reachable, mut reachable_recv) = hydroflow::util::unbounded_channel::<(usize,)>();

    seed_reachable_send.send((1,)).unwrap();
    edges_send.send((3, 4)).unwrap();
    edges_send.send((1, 2)).unwrap();
    edges_send.send((2, 5)).unwrap();

    let mut flow = datalog!(
        r#"
        .input edges `source_stream(edges)`
        .input seed_reachable `source_stream(seed_reachable)`
        .output reachable `for_each(|v| reachable.send(v).unwrap())`

        reachable(x) :- seed_reachable(x).
        reachable(y) :- reachable(x), edges(x, y).
        "#
    );

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut reachable_recv),
        &[(1,), (2,), (5,)]
    );
}

#[multiplatform_test]
pub fn test_triple_relation_join() {
    let (in1_send, in1) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (in2_send, in2) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (in3_send, in3) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize, usize, usize)>();

    in1_send.send((1, 2)).unwrap();
    in2_send.send((2, 1)).unwrap();

    in3_send.send((1, 3)).unwrap();
    in3_send.send((1, 4)).unwrap();
    in3_send.send((2, 3)).unwrap();

    let mut flow = datalog!(
        r#"
        .input in1 `source_stream(in1)`
        .input in2 `source_stream(in2)`
        .input in3 `source_stream(in3)`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(d, c, b, a) :- in1(a, b), in2(b, c), in3(c, d).
        "#
    );

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut out_recv),
        &[(3, 1, 2, 1), (4, 1, 2, 1)]
    );
}

#[multiplatform_test]
pub fn test_local_constraints() {
    let (in_send, input) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    in_send.send((1, 2)).unwrap();
    in_send.send((1, 1)).unwrap();

    let mut flow = datalog!(
        r#"
        .input input `source_stream(input)`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(x, x) :- input(x, x).
        "#
    );

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[(1, 1)]);
}

#[multiplatform_test]
pub fn test_boolean_relation_eq() {
    let (in_send, input) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    in_send.send((1, 1)).unwrap();
    in_send.send((1, 2)).unwrap();
    in_send.send((2, 1)).unwrap();

    let mut flow = datalog!(
        r#"
        .input input `source_stream(input)`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(a, b) :- input(a, b), ( a == b ).
        "#
    );

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[(1, 1)]);
}

#[multiplatform_test]
pub fn test_boolean_relation_lt() {
    let (in_send, input) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    in_send.send((1, 1)).unwrap();
    in_send.send((1, 2)).unwrap();
    in_send.send((2, 1)).unwrap();

    let mut flow = datalog!(
        r#"
        .input input `source_stream(input)`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(a, b) :- input(a, b), ( a < b ).
        "#
    );

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[(1, 2)]);
}

#[multiplatform_test]
pub fn test_boolean_relation_le() {
    let (in_send, input) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    in_send.send((1, 1)).unwrap();
    in_send.send((1, 2)).unwrap();
    in_send.send((2, 1)).unwrap();

    let mut flow = datalog!(
        r#"
        .input input `source_stream(input)`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(a, b) :- input(a, b), ( a <= b ).
        "#
    );

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut out_recv),
        &[(1, 1), (1, 2)]
    );
}

#[multiplatform_test]
pub fn test_boolean_relation_gt() {
    let (in_send, input) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    in_send.send((1, 1)).unwrap();
    in_send.send((1, 2)).unwrap();
    in_send.send((2, 1)).unwrap();

    let mut flow = datalog!(
        r#"
        .input input `source_stream(input)`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(a, b) :- input(a, b), ( a > b ).
        "#
    );

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[(2, 1)]);
}

#[multiplatform_test]
pub fn test_boolean_relation_ge() {
    let (in_send, input) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    in_send.send((1, 1)).unwrap();
    in_send.send((1, 2)).unwrap();
    in_send.send((2, 1)).unwrap();

    let mut flow = datalog!(
        r#"
        .input input `source_stream(input)`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(a, b) :- input(a, b), ( a >= b ).
        "#
    );

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut out_recv),
        &[(1, 1), (2, 1)]
    );
}

#[multiplatform_test]
pub fn test_join_multiple_and_relation() {
    let (in1_send, in1) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (in2_send, in2) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (in3_send, in3) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize, usize, usize)>();

    in1_send.send((1, 2)).unwrap();

    in2_send.send((2, 3)).unwrap();
    in2_send.send((2, 4)).unwrap();

    in3_send.send((3, 4)).unwrap();
    in3_send.send((4, 5)).unwrap();
    in3_send.send((4, 0)).unwrap();

    let mut flow = datalog!(
        r#"
        .input in1 `source_stream(in1)`
        .input in2 `source_stream(in2)`
        .input in3 `source_stream(in3)`
        .output out `for_each(|v| out.send(v).unwrap())`

        out(a, b, c, d) :- in1(a, b), in2(b, c), in3(c, d), ( d > a ).
        "#
    );
    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut out_recv),
        &[(1, 2, 3, 4), (1, 2, 4, 5)]
    );
}

#[multiplatform_test]
pub fn test_join_multiple_then_relation() {
    // Same test as test_join_multiple_and_relation, except with a filter on top instead of a
    // filter in the join.
    let (in1_send, in1) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (in2_send, in2) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (in3_send, in3) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize, usize, usize)>();

    in1_send.send((1, 2)).unwrap();

    in2_send.send((2, 3)).unwrap();
    in2_send.send((2, 4)).unwrap();

    in3_send.send((3, 4)).unwrap();
    in3_send.send((4, 5)).unwrap();
    in3_send.send((4, 0)).unwrap();

    let mut flow = datalog!(
        r#"
        .input in1 `source_stream(in1)`
        .input in2 `source_stream(in2)`
        .input in3 `source_stream(in3)`
        .output out `for_each(|v| out.send(v).unwrap())`

        int(a, b, c, d) :- in1(a, b), in2(b, c), in3(c, d).
        out(a, b, c, d) :- int(a, b, c, d), ( d > a ).
        "#
    );
    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut out_recv),
        &[(1, 2, 3, 4), (1, 2, 4, 5)]
    );
}

#[multiplatform_test]
pub fn test_next_tick() {
    let (ints_1_send, ints_1) = hydroflow::util::unbounded_channel::<(usize,)>();
    let (ints_2_send, ints_2) = hydroflow::util::unbounded_channel::<(usize,)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(usize,)>();

    ints_1_send.send((1,)).unwrap();
    ints_1_send.send((2,)).unwrap();
    ints_2_send.send((3,)).unwrap();
    ints_2_send.send((4,)).unwrap();

    let mut flow = datalog!(
        r#"
        .input ints_1 `source_stream(ints_1)`
        .input ints_2 `source_stream(ints_2)`
        .output result `for_each(|v| result.send(v).unwrap())`

        result(x) :- ints_1(x).
        result(x) :+ ints_2(x).
        "#
    );

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result_recv),
        &[(1,), (2,)]
    );

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result_recv),
        &[(3,), (4,)]
    );
}

#[multiplatform_test]
pub fn test_anti_join() {
    let (ints_1_send, ints_1) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (ints_2_send, ints_2) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (ints_3_send, ints_3) = hydroflow::util::unbounded_channel::<(usize,)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut flow = datalog!(
        r#"
        .input ints_1 `source_stream(ints_1)`
        .input ints_2 `source_stream(ints_2)`
        .input ints_3 `source_stream(ints_3)`
        .output result `for_each(|v| result.send(v).unwrap())`

        result(x, z) :- ints_1(x, y), ints_2(y, z), !ints_3(y)
        "#
    );

    ints_1_send.send((1, 2)).unwrap();
    ints_1_send.send((2, 3)).unwrap();
    ints_2_send.send((2, 3)).unwrap();
    ints_2_send.send((3, 4)).unwrap();
    ints_3_send.send((2,)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[(2, 4)]);

    ints_1_send.send((1, 2)).unwrap();
    ints_1_send.send((2, 3)).unwrap();
    ints_2_send.send((2, 3)).unwrap();
    ints_2_send.send((3, 4)).unwrap();
    ints_3_send.send((3,)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[(1, 3)]);
}

#[multiplatform_test]
pub fn test_anti_join_next_tick() {
    let (ints_1_send, ints_1) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (ints_2_send, ints_2) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (ints_3_send, ints_3) = hydroflow::util::unbounded_channel::<(usize,)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut flow = datalog!(
        r#"
        .input ints_1 `source_stream(ints_1)`
        .input ints_2 `source_stream(ints_2)`
        .input ints_3 `source_stream(ints_3)`
        .output result `for_each(|v| result.send(v).unwrap())`

        result(x, z) :+ ints_1(x, y), ints_2(y, z), !ints_3(y)
        "#
    );

    ints_1_send.send((1, 2)).unwrap();
    ints_1_send.send((2, 3)).unwrap();
    ints_2_send.send((2, 3)).unwrap();
    ints_2_send.send((3, 4)).unwrap();
    ints_3_send.send((2,)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[]);

    ints_1_send.send((1, 2)).unwrap();
    ints_1_send.send((2, 3)).unwrap();
    ints_2_send.send((2, 3)).unwrap();
    ints_2_send.send((3, 4)).unwrap();
    ints_3_send.send((3,)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[(2, 4)]);

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[(1, 3)]);
}

#[multiplatform_test]
pub fn test_anti_join_next_tick_cycle() {
    let (ints_1_send, ints_1) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (ints_2_send, ints_2) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (ints_3_send, ints_3) = hydroflow::util::unbounded_channel::<(usize,)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut flow = datalog!(
        r#"
        .input ints_1 `source_stream(ints_1)`
        .input ints_2 `source_stream(ints_2)`
        .input ints_3 `source_stream(ints_3)`
        .output result `for_each(|v| result.send(v).unwrap())`

        result(x, z) :+ ints_1(x, y), ints_2(y, z), !ints_3(y), !result(x, z)
        "#
    );

    ints_1_send.send((1, 2)).unwrap();
    ints_1_send.send((2, 3)).unwrap();
    ints_2_send.send((2, 3)).unwrap();
    ints_2_send.send((3, 4)).unwrap();
    ints_3_send.send((2,)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[]);

    ints_1_send.send((1, 2)).unwrap();
    ints_1_send.send((2, 3)).unwrap();
    ints_2_send.send((2, 3)).unwrap();
    ints_2_send.send((3, 4)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[(2, 4)]);

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[(1, 3)]);
}

#[multiplatform_test]
fn test_max() {
    let (ints_send, ints) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut flow = datalog!(
        r#"
        .input ints `source_stream(ints)`
        .output result `for_each(|v| result.send(v).unwrap())`

        result(max(a), b) :- ints(a, b)
        "#
    );

    ints_send.send((1, 2)).unwrap();
    ints_send.send((2, 2)).unwrap();
    ints_send.send((3, 2)).unwrap();

    ints_send.send((3, 3)).unwrap();
    ints_send.send((4, 3)).unwrap();
    ints_send.send((5, 3)).unwrap();

    flow.run_tick();

    let mut res = collect_ready::<Vec<_>, _>(&mut result_recv);
    res.sort_by_key(|v| v.0);
    assert_eq!(&res, &[(3, 2), (5, 3)]);
}

#[multiplatform_test]
fn test_max_all() {
    let (ints_send, ints) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut flow = datalog!(
        r#"
        .input ints `source_stream(ints)`
        .output result `for_each(|v| result.send(v).unwrap())`

        result(max(a), max(b)) :- ints(a, b)
        "#
    );

    ints_send.send((1, 3)).unwrap();
    ints_send.send((2, 2)).unwrap();
    ints_send.send((3, 1)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[(3, 3)]);
}

#[multiplatform_test]
fn test_max_next_tick() {
    let (ints_send, ints) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut flow = datalog!(
        r#"
        .input ints `source_stream(ints)`
        .output result `for_each(|v| result.send(v).unwrap())`

        result(max(a), max(b)) :+ ints(a, b)
        "#
    );

    ints_send.send((1, 3)).unwrap();
    ints_send.send((2, 2)).unwrap();
    ints_send.send((3, 1)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[]);

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[(3, 3)]);
}

#[multiplatform_test]
fn test_send_to_node() {
    let (ints_send_1, ints_1) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (_ints_send_2, ints_2) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let (result_1, mut result_recv_1) = hydroflow::util::unbounded_channel::<(usize,)>();
    let (result_2, mut result_recv_2) = hydroflow::util::unbounded_channel::<(usize,)>();

    let (_async_send_result_1, async_receive_result_1) =
        hydroflow::util::unbounded_channel::<(usize,)>();
    let (async_send_result_2, async_receive_result_2) =
        hydroflow::util::unbounded_channel::<(usize,)>();

    let mut flow_1 = {
        let ints = ints_1;
        let async_receive_result = async_receive_result_1;
        let result = result_1;

        let async_send_result = move |node: usize, data: (usize,)| {
            assert!(node == 2);
            async_send_result_2.send(data).unwrap();
        };

        datalog!(
            r#"
            .input ints `source_stream(ints)`
            .output result `for_each(|v| result.send(v).unwrap())`
            .async result `for_each(|(node, data)| async_send_result(node, data))` `source_stream(async_receive_result)`

            result@b(a) :~ ints(a, b)
            "#
        )
    };

    let mut flow_2 = {
        let ints = ints_2;
        let async_receive_result = async_receive_result_2;
        let result = result_2;

        let async_send_result = |_: usize, _: (usize,)| {
            panic!("Should not be called");
        };

        datalog!(
            r#"
            .input ints `source_stream(ints)`
            .output result `for_each(|v| result.send(v).unwrap())`
            .async result `for_each(|(node, data)| async_send_result(node, data))` `source_stream(async_receive_result)`

            result@b(a) :~ ints(a, b)
            "#
        )
    };

    ints_send_1.send((5, 2)).unwrap();

    flow_1.run_tick();
    flow_2.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv_1), &[]);
    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv_2), &[(5,)]);
}

#[multiplatform_test]
fn test_aggregations_and_comments() {
    let (ints_send, ints) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (result2, mut result_recv2) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut flow = datalog!(
        r#"
        # david doesn't think this line of code will execute
        .input ints `source_stream(ints)`
        .output result `for_each(|v| result.send(v).unwrap())`
        .output result2 `for_each(|v| result2.send(v).unwrap())`

        result(count(a), b) :- ints(a, b)
        result(sum(a), b) :+ ints(a, b)
        result2(choose(a), b) :- ints(a, b)
        "#
    );

    ints_send.send((1, 3)).unwrap();
    ints_send.send((2, 3)).unwrap();
    ints_send.send((3, 3)).unwrap();
    ints_send.send((4, 3)).unwrap();
    ints_send.send((3, 1)).unwrap();

    flow.run_tick();

    let mut res = collect_ready::<Vec<_>, _>(&mut result_recv);
    res.sort_by_key(|v| v.0);
    assert_eq!(&res, &[(1, 1), (4, 3)]);

    let mut res2 = collect_ready::<Vec<_>, _>(&mut result_recv2); // Assumes deterministic choose
    res2.sort_by_key(|v| v.0);
    assert_eq!(&res2, &[(1, 3), (3, 1)]);

    flow.run_tick();

    let mut res = collect_ready::<Vec<_>, _>(&mut result_recv);
    res.sort_by_key(|v| v.0);
    assert_eq!(&res, &[(3, 1), (10, 3)]);
}

#[multiplatform_test]
fn test_aggregations_group_by_expr() {
    let (ints_send, ints) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut flow = datalog!(
        r#"
        .input ints `source_stream(ints)`
        .output result `for_each(|v| result.send(v).unwrap())`

        result(a % 2, sum(b)) :- ints(a, b)
        "#
    );

    ints_send.send((1, 1)).unwrap();
    ints_send.send((2, 1)).unwrap();
    ints_send.send((3, 1)).unwrap();

    flow.run_tick();

    let mut res = collect_ready::<Vec<_>, _>(&mut result_recv);
    res.sort_by_key(|v| v.0);
    assert_eq!(&res, &[(0, 1), (1, 2)]);
}

#[multiplatform_test]
fn test_choose_strings() {
    let (strings_send, strings) = hydroflow::util::unbounded_channel::<(String,)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(String,)>();

    let mut flow = datalog!(
        r#"
        .input strings `source_stream(strings)`
        .output result `for_each(|v| result.send(v).unwrap())`

        result(choose(a)) :- strings(a)
        "#
    );

    strings_send.send(("hello".to_string(),)).unwrap();

    flow.run_tick();

    assert_eq!(
        &collect_ready::<Vec<_>, _>(&mut result_recv),
        &[("hello".to_string(),)]
    );
}

#[multiplatform_test]
fn test_non_copy_but_clone() {
    let (strings_send, strings) = hydroflow::util::unbounded_channel::<(String, String)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(String, String)>();

    let mut flow = datalog!(
        r#"
        .input strings `source_stream(strings)`
        .output result `for_each(|v| result.send(v).unwrap())`

        result(a, a) :- strings(a, a), strings(a, a), (a == a)
        "#
    );

    strings_send
        .send(("Hello".to_string(), "Hello".to_string()))
        .unwrap();

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result_recv),
        &[("Hello".to_string(), "Hello".to_string())]
    );
}

#[multiplatform_test]
fn test_expr_lhs() {
    let (ints_send, ints) = hydroflow::util::unbounded_channel::<(i64,)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(i64,)>();

    let mut flow = datalog!(
        r#"
        .input ints `source_stream(ints)`
        .output result `for_each(|v| result.send(v).unwrap())`

        result(123) :- ints(a)
        result(a + 123) :- ints(a)
        result(a + a) :- ints(a)
        result(123 - a) :- ints(a)
        result(123 % (a + 5)) :- ints(a)
        result(a * 5) :- ints(a)
        "#
    );

    ints_send.send((1,)).unwrap();

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result_recv),
        &[(123,), (124,), (2,), (122,), (3,), (5,)]
    );
}

#[multiplatform_test]
fn test_less_than_relation() {
    let (ints_send, ints) = hydroflow::util::unbounded_channel::<(i64,)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(i64,)>();

    let mut flow = datalog!(
        r#"
        .input ints `source_stream(ints)`
        .output result `for_each(|v| result.send(v).unwrap())`

        result(b) :- ints(a), less_than(b, a)
        "#
    );

    ints_send.send((5,)).unwrap();

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result_recv),
        &[(0,), (1,), (2,), (3,), (4,)]
    );
}

#[multiplatform_test]
fn test_expr_predicate() {
    let (ints_send, ints) = hydroflow::util::unbounded_channel::<(i64,)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(i64,)>();

    let mut flow = datalog!(
        r#"
        .input ints `source_stream(ints)`
        .output result `for_each(|v| result.send(v).unwrap())`

        result(1) :- ints(a), (a == 0)
        result(2) :- ints(a), (a != 0)
        result(3) :- ints(a), (a - 1 == 0)
        result(4) :- ints(a), (a - 1 == 1 - 1)
        "#
    );

    ints_send.send((1,)).unwrap();

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result_recv),
        &[(2,), (3,), (4,)]
    );
}

#[multiplatform_test]
fn test_persist() {
    let (ints1_send, ints1) = hydroflow::util::unbounded_channel::<(i64,)>();
    let (ints2_send, ints2) = hydroflow::util::unbounded_channel::<(i64,)>();
    let (ints3_send, ints3) = hydroflow::util::unbounded_channel::<(i64,)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(i64, i64, i64)>();
    let (result2, mut result2_recv) = hydroflow::util::unbounded_channel::<(i64,)>();
    let (result3, mut result3_recv) = hydroflow::util::unbounded_channel::<(i64,)>();
    let (result4, mut result4_recv) = hydroflow::util::unbounded_channel::<(i64,)>();

    let mut flow = datalog!(
        r#"
        .input ints1 `source_stream(ints1)`
        .persist ints1

        .input ints2 `source_stream(ints2)`
        .persist ints2

        .input ints3 `source_stream(ints3)`
        
        .output result `for_each(|v| result.send(v).unwrap())`
        .output result2 `for_each(|v| result2.send(v).unwrap())`
        .output result3 `for_each(|v| result3.send(v).unwrap())`
        .output result4 `for_each(|v| result4.send(v).unwrap())`

        result(a, b, c) :- ints1(a), ints2(b), ints3(c)
        result2(a) :- ints1(a), !ints2(a)

        intermediate(a) :- ints1(a)
        result3(a) :- intermediate(a)

        .persist intermediate_persist
        intermediate_persist(a) :- ints1(a)
        result4(a) :- intermediate_persist(a)
        "#
    );

    ints1_send.send((1,)).unwrap();
    ints2_send.send((2,)).unwrap();
    ints3_send.send((5,)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[(1, 2, 5)]);
    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result2_recv), &[(1,)]);
    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result3_recv), &[(1,)]);
    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result4_recv), &[(1,)]);

    ints2_send.send((1,)).unwrap();
    ints2_send.send((3,)).unwrap();
    ints3_send.send((6,)).unwrap();

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result_recv),
        &[(1, 2, 6), (1, 1, 6), (1, 3, 6)]
    );
    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result2_recv), &[]);
    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result3_recv), &[(1,)]);
    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result4_recv), &[(1,)]);
}

#[multiplatform_test]
fn test_persist_uniqueness() {
    let (ints2_send, ints2) = hydroflow::util::unbounded_channel::<(i64,)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(usize,)>();

    let mut flow = datalog!(
        r#"
        .persist ints1

        .input ints2 `source_stream(ints2)`
        
        ints1(a) :- ints2(a)
        
        .output result `for_each(|v| result.send(v).unwrap())`

        result(count(a)) :- ints1(a)
        "#
    );

    ints2_send.send((1,)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[(1,)]);

    ints2_send.send((1,)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[(1,)]);

    ints2_send.send((2,)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[(2,)]);
}

#[multiplatform_test]
fn test_wildcard_join_count() {
    let (ints1_send, ints1) = hydroflow::util::unbounded_channel::<(i64, i64)>();
    let (ints2_send, ints2) = hydroflow::util::unbounded_channel::<(i64,)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(usize,)>();
    let (result2, mut result2_recv) = hydroflow::util::unbounded_channel::<(usize,)>();

    let mut flow = datalog!(
        r#"
        .input ints1 `source_stream(ints1)` 
        .input ints2 `source_stream(ints2)`
        
        .output result `for_each(|v| result.send(v).unwrap())`
        .output result2 `for_each(|v| result2.send(v).unwrap())`

        result(count(*)) :- ints1(a, _), ints2(a)
        result2(count(a)) :- ints1(a, _), ints2(a)
        "#
    );

    ints1_send.send((1, 1)).unwrap();
    ints1_send.send((1, 2)).unwrap();
    ints2_send.send((1,)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[(2,)]);
    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result2_recv), &[(1,)]);
}

#[multiplatform_test]
fn test_index() {
    let (ints_send, ints) = hydroflow::util::unbounded_channel::<(i64, i64)>();
    let (result, mut result_recv) = hydroflow::util::unbounded_channel::<(i64, i64, i32)>();
    let (result2, mut result2_recv) = hydroflow::util::unbounded_channel::<(i64, usize, usize)>();

    let (result3, mut result3_recv) = hydroflow::util::unbounded_channel::<(i64, i64, usize)>();
    let (result4, mut result4_recv) = hydroflow::util::unbounded_channel::<(i64, usize, usize)>();
    let (result5, mut result5_recv) = hydroflow::util::unbounded_channel::<(i64, i64, usize)>();

    let mut flow = datalog!(
        r#"
        .input ints `source_stream(ints)` 
        
        .output result `for_each(|v| result.send(v).unwrap())`
        .output result2 `for_each(|v| result2.send(v).unwrap())`
        .output result3 `for_each(|v| result3.send(v).unwrap())`
        .output result4 `for_each(|v| result4.send(v).unwrap())`

        .persist result5
        .output result5 `for_each(|v| result5.send(v).unwrap())`

        result(a, b, index()) :- ints(a, b)
        result2(a, count(b), index()) :- ints(a, b)

        .persist ints_persisted
        ints_persisted(a, b) :- ints(a, b)

        result3(a, b, index()) :- ints_persisted(a, b)
        result4(a, count(b), index()) :- ints_persisted(a, b)
        result5(a, b, index()) :- ints_persisted(a, b)
        "#
    );

    ints_send.send((1, 1)).unwrap();
    ints_send.send((1, 2)).unwrap();
    ints_send.send((2, 1)).unwrap();

    flow.run_tick();

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result_recv),
        &[(1, 1, 0), (1, 2, 1), (2, 1, 2)]
    );

    // hashing / ordering differences?
    #[cfg(not(target_arch = "wasm32"))]
    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result2_recv),
        &[(1, 2, 0), (2, 1, 1)]
    );

    #[cfg(target_arch = "wasm32")]
    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result2_recv),
        &[(2, 1, 0), (1, 2, 1)]
    );

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result3_recv),
        &[(1, 1, 0), (1, 2, 1), (2, 1, 2)]
    );

    #[cfg(not(target_arch = "wasm32"))]
    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result4_recv),
        &[(1, 2, 0), (2, 1, 1)]
    );
    #[cfg(target_arch = "wasm32")]
    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result4_recv),
        &[(2, 1, 0), (1, 2, 1)]
    );

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result5_recv),
        &[(1, 1, 0), (1, 2, 1), (2, 1, 2)]
    );

    ints_send.send((3, 1)).unwrap();

    flow.run_tick();

    assert_eq!(&*collect_ready::<Vec<_>, _>(&mut result_recv), &[(3, 1, 0)]);
    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result2_recv),
        &[(3, 1, 0)]
    );

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result3_recv),
        &[(1, 1, 0), (1, 2, 1), (2, 1, 2), (3, 1, 3)]
    );

    #[cfg(not(target_arch = "wasm32"))]
    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result4_recv),
        &[(1, 2, 0), (2, 1, 1), (3, 1, 2)]
    );
    #[cfg(target_arch = "wasm32")]
    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result4_recv),
        &[(2, 1, 0), (3, 1, 1), (1, 2, 2)]
    );

    assert_eq!(
        &*collect_ready::<Vec<_>, _>(&mut result5_recv),
        &[(1, 1, 0), (1, 2, 1), (2, 1, 2), (3, 1, 3)]
    );
}
