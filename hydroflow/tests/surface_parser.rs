use hydroflow::hydroflow_parser;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_parser_basic() {
    hydroflow_parser! {
        reached_vertices = (merge() -> map(|v| (v, ())));
        (source_iter([0]) -> [0]reached_vertices);

        my_join = (join() -> map(|(_src, ((), dst))| dst) -> tee());
        (reached_vertices -> [0]my_join);
        (source_stream(/*(v, v) edges*/ _) -> [1]my_join);

        (my_join[0] -> [1]reached_vertices);
        (my_join[1] -> for_each(|x| println!("Reached: {}", x)));
    }

    hydroflow_parser! {
        shuffle = (merge() -> tee());
        (shuffle[0] -> [0]shuffle);
        (shuffle[1] -> [1]shuffle);
        (shuffle[2] -> [2]shuffle);
        (shuffle[3] -> [3]shuffle);
    }

    hydroflow_parser! {
        x = (map(a) -> map(b));
        (x -> x);
    }

    hydroflow_parser! {
        a = map(a); // 0
        b = (merge() -> tee()); // 1
        c = merge(); // 2
        d = tee(); // 3
        e = (merge() -> tee()); // 4
        f = map(f); // 5
        g = merge(); // 6
        h = tee(); // 7

        (a[0] -> [0]b);

        (b[0] -> [0]e);
        (b[1] -> [0]g);

        (c[0] -> [1]b);

        (d[0] -> [0]a);
        (d[1] -> [2]b);
        (d[2] -> [1]e);

        (e[0] -> [0]c);
        (e[1] -> [0]h);

        (f[0] -> [1]c);

        (g[0] -> [2]e);

        (h[0] -> [0]d);
        (h[1] -> [0]f);
        (h[2] -> [1]g);
    }
}

#[multiplatform_test]
pub fn test_parser_port_reassign() {
    hydroflow_parser! {
        id = identity();
        inn = id;
        out = id;
        out -> inn;
    };

    hydroflow_parser! {
        id = identity();
        inn = id;
        out = id;
        out[0] -> [0]inn;
    };

    hydroflow_parser! {
        id = identity();
        inn = id;
        out = id;
        [0]out[0] -> [0]inn[0]; // ?
    };
}

#[multiplatform_test]
pub fn test_parser_port_naked_basic() {
    hydroflow_parser! {
        id = identity();
        inn = [0]id;
        out = id[0];
        out -> inn;
    };
}

#[multiplatform_test]
pub fn test_parser_port_naked_knot() {
    hydroflow_parser! {
        pivot = merge() -> tee();

        inn_0 = [0]pivot;
        inn_1 = [1]pivot;

        out_0 = pivot[0];
        out_1 = pivot[1];

        out_0 -> inn_0;
        out_1 -> inn_1;
    };

    hydroflow_parser! {
        pivot = merge() -> tee();

        x_0 = [0]pivot[0];
        x_1 = [1]pivot[1];

        x_0 -> x_0;
        x_1 -> x_1;
    };

    hydroflow_parser! {
        pivot = merge() -> tee();

        x_0 = pivot[0];
        x_1 = pivot[1];

        x_0 -> [0]x_0;
        x_1 -> [1]x_1;
    };

    hydroflow_parser! {
        pivot = merge() -> tee();

        x_0 = pivot;
        x_1 = pivot;

        x_0[0] -> [0]x_0;
        x_1[1] -> [1]x_1;
    };
}

#[multiplatform_test]
pub fn test_parser_forwardref_basic() {
    hydroflow_parser! {
        source_iter(0..10) -> c;
        c = for_each(std::mem::drop);
    };
}

#[multiplatform_test]
pub fn test_parser_forwardref_chain() {
    hydroflow_parser! {
        source_iter(0..10) -> c;
        c = d;
        d = e;
        e = f;
        f = g;
        g = h;
        h = i;
        i = j;
        j = k;
        k = for_each(std::mem::drop);
    };
}

#[multiplatform_test]
pub fn test_parser_forwardref_cycle_right() {
    hydroflow_parser! {
        c = identity() -> c;
    };
}

#[multiplatform_test]
pub fn test_parser_forwardref_cycle_left() {
    hydroflow_parser! {
        c = c -> identity();
    };
}

#[multiplatform_test]
pub fn test_parser_forwardref_mutual() {
    hydroflow_parser! {
        a = identity() -> b;
        b = identity() -> a;
    };
}

#[multiplatform_test]
pub fn test_parser_forwardref_degen() {
    // TODO(mingwei):
    // This works because no links are created, so it does nothing.
    // But it would obviously be a mistake to write seriously...
    hydroflow_parser! {
        c = c;
    };
}

#[multiplatform_test]
pub fn test_parser_forwardref_tee() {
    hydroflow_parser! {
        c = c -> tee();
        c -> for_each(std::mem::drop);
    };
}

#[multiplatform_test]
pub fn test_parser_forwardref_merge() {
    hydroflow_parser! {
        c = merge() -> c;
        source_iter(0..10) -> c;
    };
}

#[multiplatform_test]
pub fn test_parser_forwardref_knot() {
    hydroflow_parser! {
        inn_0 = [0]pivot;
        inn_1 = [1]pivot;

        out_0 = pivot[0];
        out_1 = pivot[1];

        out_0 -> inn_0;
        out_1 -> inn_1;

        pivot = merge() -> tee();
    };
}

#[multiplatform_test]
pub fn test_parser_forwardref_self_middle() {
    hydroflow_parser! {
        self_ref = map(|a: usize| a) -> [0]self_ref[1] -> map(|b: usize| b);
    };
}
