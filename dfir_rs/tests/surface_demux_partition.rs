use dfir_rs::dfir_syntax;
use dfir_rs::util::collect_ready;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_demux_1() {
    enum Shape {
        Circle(f64),
        Rectangle { width: f64, height: f64 },
        Square(f64),
    }

    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut df = dfir_syntax! {
        my_demux = source_iter([
            Shape::Circle(5.0),
            Shape::Rectangle { width: 10.0, height: 8.0 },
            Shape::Square(9.0),
        ]) -> demux(|shape, var_args!(circ, rect)| {
            match shape {
                Shape::Circle(radius) => circ.give(radius),
                Shape::Rectangle { width, height } => rect.give((width, height)),
                Shape::Square(side) => rect.give((side, side)),
            }
        });

        out = union() -> map(|float| (float * 1000.0).trunc() as u32) -> for_each(|area| out_send.send(area).unwrap());

        my_demux[circ] -> map(|r| std::f64::consts::PI * r * r) -> out;
        my_demux[rect] -> map(|(w, h)| w * h) -> out;
    };
    df.run_available_sync();

    assert_eq!(
        &[78_539, 80_000, 81_000],
        &*collect_ready::<Vec<_>, _>(&mut out_recv)
    );
}

#[multiplatform_test]
pub fn test_demux_fizzbuzz_1() {
    let mut df = dfir_syntax! {
        my_demux = source_iter(1..=100)
            -> demux(|v, var_args!(fzbz, fizz, buzz, vals)|
                match v {
                    v if 0 == v % 15 => fzbz.give(()),
                    v if 0 == v % 3 => fizz.give(()),
                    v if 0 == v % 5 => buzz.give(()),
                    v => vals.give(v),
                }
            );
        my_demux[fzbz] -> for_each(|_| println!("fizzbuzz"));
        my_demux[fizz] -> for_each(|_| println!("fizz"));
        my_demux[buzz] -> for_each(|_| println!("buzz"));
        my_demux[vals] -> for_each(|x| println!("{}", x));
    };
    df.run_available_sync();
}

#[multiplatform_test]
pub fn test_demux_fizzbuzz_2() {
    let mut df = dfir_syntax! {
        my_demux = source_iter(1..=100)
        -> demux(|v, var_args!(fzbz, fizz, buzz, vals)|
            match (v % 3, v % 5) {
                (0, 0) => fzbz.give(()),
                (0, _) => fizz.give(()),
                (_, 0) => buzz.give(()),
                (_, _) => vals.give(v),
            }
        );
        my_demux[fzbz] -> for_each(|_| println!("fizzbuzz"));
        my_demux[fizz] -> for_each(|_| println!("fizz"));
        my_demux[buzz] -> for_each(|_| println!("buzz"));
        my_demux[vals] -> for_each(|x| println!("{}", x));
    };
    df.run_available_sync();
}

#[multiplatform_test]
pub fn test_partition_fizzbuzz() {
    let mut df = dfir_syntax! {
        my_partition = source_iter(1..=100)
            -> partition(|&v, [fzbz, fizz, buzz, vals]|
                match (v % 3, v % 5) {
                    (0, 0) => fzbz,
                    (0, _) => fizz,
                    (_, 0) => buzz,
                    (_, _) => vals,
                }
            );
        my_partition[vals] -> for_each(|x| println!("{}", x));
        my_partition[fizz] -> for_each(|_| println!("fizz"));
        my_partition[buzz] -> for_each(|_| println!("buzz"));
        my_partition[fzbz] -> for_each(|_| println!("fizzbuzz"));
    };
    df.run_available_sync();
}

#[multiplatform_test]
pub fn test_partition_round() {
    let mut df = dfir_syntax! {
        my_partition = source_iter(0..100)
            -> partition(|v, len| v % len);
        my_partition[2] -> for_each(|x| println!("{} 2", x));
        my_partition[1] -> for_each(|x| println!("{} 1", x));
        my_partition[3] -> for_each(|x| println!("{} 3", x));
        my_partition[0] -> for_each(|x| println!("{} 0", x));
    };
    df.run_available_sync();
}
