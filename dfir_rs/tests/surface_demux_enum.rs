use dfir_rs::dfir_syntax;
use dfir_rs::util::collect_ready;
use dfir_rs::util::demux_enum::{DemuxEnum, DemuxEnumBase};
use multiplatform_test::multiplatform_test;
use pusherator::for_each::ForEach;

#[multiplatform_test]
fn test_manual_impl() {
    use pusherator::Pusherator;

    let (out_send, out_recv) = dfir_rs::util::unbounded_channel();

    enum Shape {
        Square(usize),
        Rectangle { w: usize, h: usize },
        Circle { r: usize },
    }
    impl<Square, Rectangle, Circle> DemuxEnum<(Square, Rectangle, Circle)> for Shape
    where
        Square: Pusherator<Item = usize>,
        Rectangle: Pusherator<Item = (usize, usize)>,
        Circle: Pusherator<Item = (usize,)>,
    {
        fn demux_enum(self, (sq, re, ci): &mut (Square, Rectangle, Circle)) {
            match self {
                Self::Square(s) => sq.give(s),
                Self::Rectangle { w, h } => re.give((w, h)),
                Self::Circle { r } => ci.give((r,)),
            }
        }
    }
    impl DemuxEnumBase for Shape {}

    let vals = [
        Shape::Square(5),
        Shape::Rectangle { w: 5, h: 6 },
        Shape::Circle { r: 6 },
    ];
    let mut nexts = (
        ForEach::new(|x| out_send.send(format!("1 {:?}", x)).unwrap()),
        ForEach::new(|x| out_send.send(format!("2 {:?}", x)).unwrap()),
        ForEach::new(|x| out_send.send(format!("3 {:?}", x)).unwrap()),
    );
    for val in vals {
        val.demux_enum(&mut nexts);
    }
    assert_eq!(
        &["1 5", "2 (5, 6)", "3 (6,)"],
        &*collect_ready::<Vec<_>, _>(out_recv)
    )
}

#[multiplatform_test]
fn test_derive() {
    let (out_send, out_recv) = dfir_rs::util::unbounded_channel();

    #[derive(DemuxEnum)]
    enum Shape {
        Square(usize),
        Rectangle { w: usize, h: usize },
        Circle { r: usize },
    }

    let vals = [
        Shape::Circle { r: 6 },
        Shape::Rectangle { w: 5, h: 6 },
        Shape::Square(5),
    ];
    let mut nexts = (
        ForEach::new(|x| out_send.send(format!("1 {:?}", x)).unwrap()),
        ForEach::new(|x| out_send.send(format!("2 {:?}", x)).unwrap()),
        ForEach::new(|x| out_send.send(format!("3 {:?}", x)).unwrap()),
    );
    for val in vals {
        val.demux_enum(&mut nexts);
    }
    assert_eq!(
        &["1 (6,)", "2 (5, 6)", "3 (5,)",],
        &*collect_ready::<Vec<_>, _>(out_recv)
    )
}

#[multiplatform_test]
pub fn test_demux_enum() {
    let (out_send, out_recv) = dfir_rs::util::unbounded_channel();

    #[derive(DemuxEnum)]
    enum Shape {
        Square(f64),
        Rectangle { w: f64, h: f64 },
        Circle { r: f64 },
    }

    let mut df = dfir_syntax! {
        my_demux = source_iter([
            Shape::Square(9.0),
            Shape::Rectangle { w: 10.0, h: 8.0 },
            Shape::Circle { r: 5.0 },
        ]) -> demux_enum::<Shape>();

        my_demux[Square] -> map(|(s,)| s * s) -> out;
        my_demux[Circle] -> map(|(r,)| std::f64::consts::PI * r * r) -> out;
        my_demux[Rectangle] -> map(|(w, h)| w * h) -> out;

        out = union()
            -> map(|area| format!("{:.2}", area))
            -> for_each(|area_str| out_send.send(area_str).unwrap());
    };
    df.run_available_sync();

    let areas = collect_ready::<Vec<_>, _>(out_recv);
    assert_eq!(&["81.00", "78.54", "80.00"], &*areas);
}

#[multiplatform_test]
pub fn test_demux_enum_generic() {
    #[derive(DemuxEnum)]
    enum Shape<N> {
        Square(N),
        Rectangle { w: N, h: N },
        Circle { r: N },
    }

    fn test<N>(s: N, w: N, h: N, r: N, expected: &[&str])
    where
        N: 'static + Into<f64>,
    {
        let (out_send, out_recv) = dfir_rs::util::unbounded_channel();

        let mut df = dfir_syntax! {
            my_demux = source_iter([
                Shape::Square(s),
                Shape::Rectangle { w, h },
                Shape::Circle { r },
            ]) -> demux_enum::<Shape<N>>();

            my_demux[Square] -> map(|(s,)| s.into()) -> map(|s| s * s) -> out;
            my_demux[Circle] -> map(|(r,)| r.into()) -> map(|r| std::f64::consts::PI * r * r) -> out;
            my_demux[Rectangle] -> map(|(w, h)| w.into() * h.into()) -> out;

            out = union()
                -> map(|area| format!("{:.2}", area))
                -> for_each(|area_str| out_send.send(area_str).unwrap());
        };
        df.run_available_sync();

        let areas = collect_ready::<Vec<_>, _>(out_recv);
        assert_eq!(expected, &*areas);
    }
    test::<f32>(9., 10., 8., 5., &["81.00", "78.54", "80.00"]);
    test::<u32>(9, 10, 8, 5, &["81.00", "78.54", "80.00"]);
}

#[multiplatform_test]
fn test_zero_variants() {
    #[derive(DemuxEnum)]
    enum Never {}

    let mut df = dfir_syntax! {
        source_iter(std::iter::empty::<Never>()) -> demux_enum::<Never>();
    };
    df.run_available_sync();
}

#[multiplatform_test]
fn test_one_variant() {
    #[derive(DemuxEnum)]
    enum Request<T> {
        OnlyMessage(T),
    }

    let (out_send, out_recv) = dfir_rs::util::unbounded_channel();

    let mut df = dfir_syntax! {
        input = source_iter([Request::OnlyMessage("hi")]) -> demux_enum::<Request<&'static str>>();
        input[OnlyMessage] -> for_each(|(msg,)| out_send.send(msg).unwrap());
    };
    df.run_available_sync();

    assert_eq!(&["hi"], &*collect_ready::<Vec<_>, _>(out_recv));
}
