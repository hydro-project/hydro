use dfir_rs::dfir_syntax;

fn main() {
    struct Shape {
        area: f64,
    }

    let mut df = dfir_syntax! {
        my_demux = source_iter([
            Shape { area: 10.0 },
            Shape { area: 9.0 },
        ]) -> demux_enum::<Shape>();
        my_demux[Rectangle] -> for_each(std::mem::drop);
        my_demux[Circle] -> for_each(std::mem::drop);
        my_demux[Square] -> for_each(std::mem::drop);
        my_demux[Ellipse] -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
